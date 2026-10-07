import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { Captions } from './captions.svelte'

describe('Captions', () => {
  beforeEach(() => vi.useFakeTimers())
  afterEach(() => vi.useRealTimers())

  it('shows each caption for its duration, extending one that repeats', () => {
    const captions = new Captions()
    captions.show('a bell', 3000)
    captions.show('rain', 4000)
    vi.advanceTimersByTime(2000)
    captions.show('a bell', 3000)
    expect(captions.current.map((c) => c.text)).toEqual(['a bell', 'rain'])

    vi.advanceTimersByTime(2999)
    expect(captions.current.map((c) => c.text)).toEqual(['a bell'])
    vi.advanceTimersByTime(1)
    expect(captions.current).toEqual([])
  })

  it('clears captions and the soundscape', () => {
    const captions = new Captions()
    captions.show('a bell', 3000)
    captions.soundscape = ['rain']
    captions.clear()
    vi.advanceTimersByTime(3000)
    expect([captions.current, captions.soundscape]).toEqual([[], []])
  })
})
