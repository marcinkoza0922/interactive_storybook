#!/usr/bin/env python3
"""Synthesize placeholder narration for the sample book (examples/the-uneven-bell/assets/voice/)
and for the starter project that `tome new` creates (cli/templates/voice/).

Requires espeak-ng and ffmpeg (with libopus). The voices are robotic on purpose: they stand in
for recorded narration while exercising the narration queue, the page-turn guard and text
highlighting.

Lines are spoken phrase by phrase (split at punctuation) and joined with short pauses, so
each phrase's exact start and end are known; those go into a WebVTT timing file beside the
recording, which `tome` aligns to the text for word-by-word highlighting. Lines marked
`timing=False` get no timing file, so the sample also shows whole-paragraph highlighting.
"""

import pathlib
import re
import subprocess
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
SAMPLE = ROOT / 'examples' / 'the-uneven-bell' / 'assets' / 'voice'
STARTER = ROOT / 'cli' / 'templates' / 'voice'
RATE = 24000
PAUSE_S = 0.18

# name: (espeak-ng voice, words per minute, pitch 0–99, text, write a timing file)
STARTER_LINES = {
    'oh-how-wonderful': ('en-gb+f3', 150, 60, 'Oh, how wonderful.', True),
}

SAMPLE_LINES = {
    'dockmaster': ('en-gb', 150, 30, "If you're waiting for someone, they're not coming. Nobody meets the evening boat.", False),
    'the-address': (
        'en-gb+f3',
        160,
        55,
        'She found the address on the letter at the end of a crooked lane: a narrow house with its shutters '
        'nailed closed. The door, when she tried it, swung open at a touch.',
        True,
    ),
    'the-note': ('en-gb+f4', 130, 60, 'You came after all.', True),
    'the-daughter': ('en-gb', 145, 25, "You'll be the daughter.", True),
    'which-daughter': ('en-gb+f3', 140, 60, 'Which daughter?', True),
}


def phrases(text: str) -> list[str]:
    """Split after commas, colons, semicolons and sentence ends."""
    return [p.strip() for p in re.split(r'(?<=[,:;.?!])\s+', text) if p.strip()]


def speak(voice: str, speed: int, pitch: int, text: str, wav: pathlib.Path) -> float:
    """Synthesize `text` to `wav`, trimmed of leading and trailing silence; returns its length."""
    raw = wav.with_suffix('.raw.wav')
    subprocess.run(['espeak-ng', '-v', voice, '-s', str(speed), '-p', str(pitch), '-w', str(raw), text], check=True)
    trim = 'silenceremove=start_periods=1:start_threshold=-50dB'
    subprocess.run(
        ['ffmpeg', '-y', '-loglevel', 'error', '-i', str(raw), '-af', f'{trim},areverse,{trim},areverse', '-ar', str(RATE), '-ac', '1', str(wav)],
        check=True,
    )
    return float(subprocess.run(
        ['ffprobe', '-v', 'error', '-show_entries', 'format=duration', '-of', 'csv=p=0', str(wav)],
        check=True, capture_output=True, text=True,
    ).stdout)


def stamp(seconds: float) -> str:
    minutes, seconds = divmod(seconds, 60)
    return f'{int(minutes):02d}:{seconds:06.3f}'


def generate(out: pathlib.Path, lines: dict) -> None:
    out.mkdir(parents=True, exist_ok=True)
    for name, (voice, speed, pitch, text, timed) in lines.items():
        with tempfile.TemporaryDirectory() as tmp:
            tmp = pathlib.Path(tmp)
            cues, parts, at = [], [], 0.0
            for index, phrase in enumerate(phrases(text)):
                wav = tmp / f'{index}.wav'
                length = speak(voice, speed, pitch, phrase, wav)
                cues.append(f'{stamp(at)} --> {stamp(at + length)}\n{phrase}')
                parts.append(wav)
                at += length + PAUSE_S

            # Join the phrases with a short pause after each.
            inputs = [arg for wav in parts for arg in ('-i', str(wav))]
            filters = ''.join(f'[{i}]apad=pad_dur={PAUSE_S}[p{i}];' for i in range(len(parts)))
            joined = ''.join(f'[p{i}]' for i in range(len(parts)))
            target = out / f'{name}.ogg'
            subprocess.run(
                ['ffmpeg', '-y', '-loglevel', 'error', *inputs, '-filter_complex', f'{filters}{joined}concat=n={len(parts)}:v=0:a=1',
                 '-c:a', 'libopus', '-b:a', '32k', str(target)],
                check=True,
            )

        timing = out / f'{name}.vtt'
        if timed:
            timing.write_text('WEBVTT\n\n' + '\n\n'.join(cues) + '\n')
        elif timing.exists():
            timing.unlink()
        print(f'{target.name}  {target.stat().st_size // 1024} KB' + (f'  + {timing.name} ({len(cues)} phrases)' if timed else ''))


def main() -> None:
    generate(SAMPLE, SAMPLE_LINES)
    generate(STARTER, STARTER_LINES)


if __name__ == '__main__':
    main()
