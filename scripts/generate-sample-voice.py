#!/usr/bin/env python3
"""Synthesize placeholder narration for the sample book (examples/the-uneven-bell/assets/voice/).

Requires espeak-ng and ffmpeg (with libopus). The voices are robotic on purpose: they stand in
for recorded narration while exercising the narration queue and the page-turn guard.
"""

import pathlib
import subprocess
import tempfile

OUT = pathlib.Path(__file__).resolve().parent.parent / 'examples' / 'the-uneven-bell' / 'assets' / 'voice'

# name: (espeak-ng voice, words per minute, pitch 0–99, text)
LINES = {
    'dockmaster': ('en-gb', 150, 30, "If you're waiting for someone, they're not coming. Nobody meets the evening boat."),
    'the-address': (
        'en-gb+f3',
        160,
        55,
        'She found the address on the letter at the end of a crooked lane: a narrow house with its shutters '
        'nailed closed. The door, when she tried it, swung open at a touch.',
    ),
    'the-note': ('en-gb+f4', 130, 60, 'You came after all.'),
    'the-daughter': ('en-gb', 145, 25, "You'll be the daughter."),
    'which-daughter': ('en-gb+f3', 140, 60, 'Which daughter?'),
}


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    for name, (voice, speed, pitch, text) in LINES.items():
        target = OUT / f'{name}.ogg'
        with tempfile.NamedTemporaryFile(suffix='.wav') as wav:
            subprocess.run(['espeak-ng', '-v', voice, '-s', str(speed), '-p', str(pitch), '-w', wav.name, text], check=True)
            subprocess.run(
                ['ffmpeg', '-y', '-loglevel', 'error', '-i', wav.name, '-c:a', 'libopus', '-b:a', '32k', str(target)],
                check=True,
            )
        print(f'{target.name}  {target.stat().st_size // 1024} KB')


if __name__ == '__main__':
    main()
