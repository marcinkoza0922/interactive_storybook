import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Block, Bundle, Cue } from '../bundle/types'
import { advance, arriveBackward, back, type ReaderState } from '../reader/navigation'
import { AudioDirector } from './director'
import type { AudioEngine } from './engine'
import { pageEndStates } from './state'

class FakeEngine implements AudioEngine {
  calls: string[] = []
  setMusic(voice: { src: string } | null, fadeMs: number) {
    this.calls.push(`music ${voice?.src ?? 'off'} ${fadeMs}`)
  }
  setAmbient(id: string, voice: { src: string } | null, fadeMs: number) {
    this.calls.push(`ambient ${id} ${voice?.src ?? 'off'} ${fadeMs}`)
  }
  playSfx(src: string) {
    this.calls.push(`sfx ${src}`)
  }
  preload() {}
  setChannelVolume() {}
  take() {
    const calls = this.calls
    this.calls = []
    return calls
  }
}

const block = (id: string, cues: Cue[], step?: number): Block => ({
  id,
  html: '',
  cues,
  ...(step ? { reveal: { step, effect: 'fade' as const } } : {}),
})

// ch1 p0: music A + rain.   ch1 p1: thunder sfx, then on reveal step 1 music B (delayed 500ms).
// ch2 p0: rain stops, hard music stop.
const bundle: Bundle = {
  bundle_schema_version: 1,
  book: { id: 't', title: 'T', author: 'A', language: 'en' },
  chapters: [
    {
      id: 'one',
      title: 'One',
      content_hash: 'h',
      pages: [
        { blocks: [block('a', [{ kind: 'music', src: 'A' }, { kind: 'ambient', id: 'rain', src: 'rain' }])] },
        {
          blocks: [
            block('b', [{ kind: 'sfx', src: 'thunder' }]),
            block('c', [{ kind: 'music', src: 'B', fade_ms: 3000, delay_ms: 500 }], 1),
          ],
        },
      ],
    },
    {
      id: 'two',
      title: 'Two',
      content_hash: 'h',
      pages: [{ blocks: [block('d', [{ kind: 'ambient_stop', id: 'rain' }, { kind: 'music_stop', fade_ms: 0 }])] }],
    },
  ],
}

const start: ReaderState = { position: { chapter: 0, page: 0 }, revealed: 0, direction: 'forward' }

describe('AudioDirector', () => {
  let engine: FakeEngine
  let director: AudioDirector
  let reader: ReaderState

  const step = (next: ReaderState | null, change: 'turn' | 'reveal') => {
    reader = next!
    director.update(reader, change)
  }

  beforeEach(() => {
    vi.useFakeTimers()
    engine = new FakeEngine()
    director = new AudioDirector(bundle, engine)
    reader = start
  })

  afterEach(() => vi.useRealTimers())

  it('starts the first page cues on begin', () => {
    director.update(start, 'enter')
    expect(engine.take()).toEqual(['music A 1000', 'ambient rain rain 1500'])
  })

  it('plays sound effects on arrival and honours cue delay and fade on reveal', () => {
    director.update(start, 'enter')
    engine.take()

    step(advance(bundle, reader), 'turn')
    expect(engine.take()).toEqual(['sfx thunder'])

    step(advance(bundle, reader), 'reveal')
    expect(engine.take()).toEqual([])
    vi.advanceTimersByTime(500)
    expect(engine.take()).toEqual(['music B 3000'])
  })

  it('keeps music playing across a chapter boundary until stopped, then stops as cued', () => {
    director.update(arriveBackward(bundle, { chapter: 0, page: 1 }), 'enter')
    expect(engine.take()).toEqual(['music B 1000', 'ambient rain rain 1500'])

    step(advance(bundle, arriveBackward(bundle, { chapter: 0, page: 1 })), 'turn')
    expect(engine.take()).toEqual(['music off 0', 'ambient rain off 1500'])
  })

  it('going back plays no sound effects and restores music only after the delay', () => {
    director.update(arriveBackward(bundle, { chapter: 1, page: 0 }), 'enter')
    engine.take()

    step(back(bundle, { position: { chapter: 1, page: 0 }, revealed: 0, direction: 'forward' }), 'turn')
    vi.advanceTimersByTime(2999)
    expect(engine.take()).toEqual([])
    vi.advanceTimersByTime(1)
    expect(engine.take()).toEqual(['music B 1000', 'ambient rain rain 1500'])
  })

  it('a quick glance back and forward again leaves the music untouched', () => {
    director.update(start, 'enter')
    step(advance(bundle, reader), 'turn') // page 2: thunder, music A continues
    engine.take()

    step(back(bundle, reader), 'turn')
    vi.advanceTimersByTime(1000) // back again before the restore delay
    step(advance(bundle, reader), 'turn')
    vi.advanceTimersByTime(10_000)
    expect(engine.take()).toEqual(['sfx thunder'])
  })

  it('returning forward to a page undoes cues from steps that are no longer revealed', () => {
    reader = arriveBackward(bundle, { chapter: 0, page: 1 })
    director.update(reader, 'enter') // fully revealed: music B
    engine.take()

    step(back(bundle, reader), 'turn')
    step(advance(bundle, reader), 'turn') // step 1 unrevealed again, so music A
    expect(engine.take()).toEqual(['sfx thunder', 'music A 1000'])
  })

  it('drops delayed cues of a page the reader has left', () => {
    director.update(arriveBackward(bundle, { chapter: 0, page: 0 }), 'enter')
    step(advance(bundle, reader), 'turn')
    step(advance(bundle, reader), 'reveal')
    engine.take()

    step(advance(bundle, reader), 'turn') // leave before music B's 500ms delay elapses
    vi.advanceTimersByTime(10_000)
    expect(engine.take()).toEqual(['music off 0', 'ambient rain off 1500'])
  })

  it('stop fades everything out', () => {
    director.update(start, 'enter')
    engine.take()
    director.stop()
    expect(engine.take()).toEqual(['music off 2000', 'ambient rain off 1500'])
  })
})

describe('pageEndStates', () => {
  it('carries music and ambience across pages and chapters', () => {
    const states = pageEndStates(bundle)
    expect(states[0][0]).toEqual({ music: { src: 'A', volume: 1 }, ambient: { rain: { src: 'rain', volume: 1 } } })
    expect(states[0][1].music?.src).toBe('B')
    expect(states[1][0]).toEqual({ music: null, ambient: {} })
  })
})
