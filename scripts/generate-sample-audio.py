#!/usr/bin/env python3
"""Synthesize placeholder audio for the sample book (examples/the-uneven-bell/assets/audio/).

Requires numpy and ffmpeg (with libopus). The output is deliberately simple:
it exists to exercise the audio engine, not to sound good.
"""

import pathlib
import subprocess
import tempfile
import wave

import numpy as np

RATE = 44100
OUT = pathlib.Path(__file__).resolve().parent.parent / 'examples' / 'the-uneven-bell' / 'assets' / 'audio'
rng = np.random.default_rng(7)


def periodic_freq(hz: float, seconds: float) -> float:
    """Round a frequency so it completes whole cycles in `seconds`, making the loop seamless."""
    return round(hz * seconds) / seconds


def pad(notes_hz: list[float], seconds: float, level: float = 0.18) -> np.ndarray:
    t = np.arange(int(RATE * seconds)) / RATE
    swell = periodic_freq(0.125, seconds)
    signal = sum(
        np.sin(2 * np.pi * periodic_freq(f, seconds) * t) * (0.7 + 0.3 * np.sin(2 * np.pi * swell * t + i))
        for i, f in enumerate(notes_hz)
    )
    return level * signal / len(notes_hz)


def looped_noise(seconds: float, color: float, swell_hz: float, level: float) -> np.ndarray:
    """Coloured noise with a slow swell, crossfaded end-into-start so it loops without a seam."""
    blend = int(RATE * 1.5)
    n = int(RATE * seconds)
    spectrum = np.fft.rfft(rng.standard_normal(n + blend))
    freqs = np.fft.rfftfreq(n + blend, 1 / RATE)
    spectrum /= np.maximum(freqs, 20) ** color
    noise = np.fft.irfft(spectrum, n + blend)
    noise /= np.abs(noise).max()

    ramp = np.linspace(0, 1, blend)
    noise[:blend] = noise[:blend] * ramp + noise[n:] * (1 - ramp)
    noise = noise[:n]
    t = np.arange(n) / RATE
    return level * noise * (0.6 + 0.4 * np.sin(2 * np.pi * periodic_freq(swell_hz, seconds) * t))


def bell(base_hz: float, seconds: float) -> np.ndarray:
    t = np.arange(int(RATE * seconds)) / RATE
    partials = [(0.56, 1.0), (0.92, 0.7), (1.19, 0.6), (1.71, 0.4), (2.0, 0.35), (2.74, 0.25), (3.76, 0.15)]
    signal = sum(a * np.sin(2 * np.pi * base_hz * r * t) * np.exp(-t * (1.2 + r)) for r, a in partials)
    return 0.35 * signal / sum(a for _, a in partials)


def match_strike(seconds: float) -> np.ndarray:
    n = int(RATE * seconds)
    t = np.arange(n) / RATE
    burst = rng.standard_normal(n) * np.exp(-t * 18)
    flare = np.convolve(rng.standard_normal(n), np.ones(40) / 40, 'same') * np.exp(-t * 4) * 0.6
    return 0.4 * (burst + flare) / np.abs(burst + flare).max()


def write(name: str, samples: np.ndarray) -> None:
    pcm = (np.clip(samples, -1, 1) * 32767).astype(np.int16)
    with tempfile.NamedTemporaryFile(suffix='.wav') as tmp:
        with wave.open(tmp.name, 'wb') as w:
            w.setnchannels(1)
            w.setsampwidth(2)
            w.setframerate(RATE)
            w.writeframes(pcm.tobytes())
        target = OUT / f'{name}.ogg'
        subprocess.run(
            ['ffmpeg', '-y', '-loglevel', 'error', '-i', tmp.name, '-c:a', 'libopus', '-b:a', '48k', str(target)],
            check=True,
        )
        print(f'{target.name}  {target.stat().st_size // 1024} KB')


if __name__ == '__main__':
    OUT.mkdir(parents=True, exist_ok=True)
    write('harbour', pad([110.0, 164.81, 220.0, 277.18], 24))
    write('letter', pad([146.83, 174.61, 220.0, 293.66], 20))
    write('keeper', pad([65.41, 98.0, 130.81, 155.56], 20, level=0.22))
    write('sea', looped_noise(14, color=1.0, swell_hz=0.15, level=0.35))
    write('wind', looped_noise(12, color=0.6, swell_hz=0.2, level=0.18))
    write('bell', bell(392.0, 4.5))
    write('match', match_strike(0.8))
