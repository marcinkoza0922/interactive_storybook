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
  /** Lines of narration started, to finish from the test with end(). */
  lines: { src: string; onEnded: () => void; stopped: boolean }[] = []
  /** The most lines ever audible at once: narration must never overlap. */
  maxConcurrent = 0
  playVoice(src: string, _volume: number, onEnded: () => void) {
    this.calls.push(`voice ${src}`)
    const line = { src, onEnded, stopped: false }
    this.lines.push(line)
    this.maxConcurrent = Math.max(this.maxConcurrent, this.lines.filter((l) => !l.stopped).length)
    return {
      stop: () => {
        line.stopped = true
        this.calls.push(`voice stop ${src}`)
      },
    }
  }
  /** Ducking changes, kept apart from `calls` so other tests needn't mention them. */
  ducks: string[] = []
  setDucking(level: number, fadeMs: number) {
    this.ducks.push(`${level} ${fadeMs}`)
  }
  /** Each line lasts a second per letter of its name, so tests can tell them apart. */
  voiceDuration(src: string) {
    return Promise.resolve(src.length * 1000)
  }
  /** Finish the line playing now. */
  end() {
    const line = this.lines.filter((l) => !l.stopped).at(-1)!
    line.stopped = true
    line.onEnded()
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

describe('narration', () => {
  const voice = (src: string, delay_ms?: number): Cue => ({ kind: 'voice', src, ...(delay_ms ? { delay_ms } : {}) })

  // p0: line A on arrival; reveal step 1 has line BB (after a 500ms pause) then C.   p1: line D.
  const narrated: Bundle = {
    ...bundle,
    chapters: [
      {
        id: 'one',
        title: 'One',
        content_hash: 'h',
        pages: [
          { blocks: [block('a', [voice('A')]), block('b', [voice('BB', 500)], 1), block('c', [voice('C')], 1)] },
          { blocks: [block('d', [voice('D')])] },
        ],
      },
    ],
  }

  let engine: FakeEngine
  let narrating: boolean[]
  let enabled: boolean
  let director: AudioDirector
  let reader: ReaderState

  const step = (next: ReaderState | null, change: 'turn' | 'reveal') => {
    reader = next!
    director.update(reader, change)
  }
  const page2: ReaderState = { position: { chapter: 0, page: 1 }, revealed: 0, direction: 'forward' }

  beforeEach(() => {
    vi.useFakeTimers()
    engine = new FakeEngine()
    narrating = []
    enabled = true
    director = new AudioDirector(narrated, engine, { narrationEnabled: () => enabled, onNarrationChange: (n) => narrating.push(n) })
    reader = start
  })

  afterEach(() => vi.useRealTimers())

  it('plays lines strictly in order, one at a time, with delays as pauses before their turn', () => {
    director.update(start, 'enter')
    step(advance(narrated, reader), 'reveal') // queues BB (500ms pause) then C behind A
    expect(engine.take()).toEqual(['voice A'])

    engine.end()
    expect(engine.take(), 'BB waits out its pause first').toEqual([])
    vi.advanceTimersByTime(499)
    expect(engine.take()).toEqual([])
    vi.advanceTimersByTime(1)
    expect(engine.take()).toEqual(['voice BB'])

    engine.end()
    expect(engine.take()).toEqual(['voice C'])
    engine.end()
    expect(director.narrating).toBe(false)
    expect(engine.maxConcurrent).toBe(1)
    expect(narrating).toEqual([true, false])
  })

  it('cuts off the line in progress, even mid-pause, when the page turns', () => {
    director.update(start, 'enter')
    step(advance(narrated, reader), 'reveal')
    engine.end() // A done; BB is in its pause
    engine.take()
    step(page2, 'turn')
    vi.advanceTimersByTime(5000)
    expect(engine.take(), 'BB and C never play').toEqual(['voice D'])
  })

  it('reads the whole page from its first line when resuming it', () => {
    // Resuming lands on the page fully revealed: A, then BB after its pause, then C.
    director.update(arriveBackward(narrated, { chapter: 0, page: 0 }), 'enter')
    expect(engine.take()).toEqual(['voice A'])
    engine.end()
    vi.advanceTimersByTime(500)
    expect(engine.take()).toEqual(['voice BB'])
    engine.end()
    expect(engine.take()).toEqual(['voice C'])
  })

  it('plays nothing going back, and stops narration there', () => {
    director.update(start, 'enter')
    step(page2, 'turn')
    engine.take()
    step(back(narrated, reader), 'turn')
    vi.advanceTimersByTime(5000)
    expect(engine.take()).toEqual(['voice stop D'])
    expect(director.narrating).toBe(false)
  })

  it('passes lines silently, each as long as its recording, when narration is off', async () => {
    enabled = false
    director.update(start, 'enter')
    step(advance(narrated, reader), 'reveal')
    expect(director.narrating).toBe(true)

    // A (1s), then BB's 500ms pause and its 2s, then C (1s): 4.5s of silent narration.
    await vi.advanceTimersByTimeAsync(4400)
    expect(director.narrating).toBe(true)
    await vi.advanceTimersByTimeAsync(200)
    expect(director.narrating).toBe(false)
    expect(engine.take(), 'nothing is played').toEqual([])
  })

  it('a page turn also ends silent narration', async () => {
    enabled = false
    director.update(start, 'enter')
    await vi.advanceTimersByTimeAsync(100)
    step(page2, 'turn')
    await vi.advanceTimersByTimeAsync(500)
    expect(director.narrating, 'D is passing silently').toBe(true)
    await vi.advanceTimersByTimeAsync(600)
    expect(director.narrating).toBe(false)
  })
})

describe('narration progress', () => {
  beforeEach(() => vi.useFakeTimers())
  afterEach(() => vi.useRealTimers())

  const book: Bundle = {
    ...bundle,
    chapters: [
      {
        id: 'one',
        title: 'One',
        content_hash: 'h',
        pages: [
          {
            blocks: [
              block('a', [{ kind: 'voice', src: 'A', words: [0, 400] }]),
              block('b', [{ kind: 'voice', src: 'B', delay_ms: 300 }], 1),
            ],
          },
        ],
      },
    ],
  }

  it('reports the paragraph and words of the line being heard, and nothing in between', () => {
    const engine = new FakeEngine()
    const director = new AudioDirector(book, engine)
    director.update(start, 'enter')
    director.update({ ...start, revealed: 1 }, 'reveal')
    expect(director.narrationProgress()).toMatchObject({ blockId: 'a', words: [0, 400] })

    engine.end()
    expect(director.narrationProgress(), 'B is waiting out its delay').toBeNull()
    vi.advanceTimersByTime(300)
    expect(director.narrationProgress()).toMatchObject({ blockId: 'b', words: undefined })
    engine.end()
    expect(director.narrationProgress()).toBeNull()
  })
})

describe('ducking', () => {
  const voice = (src: string, delay_ms?: number): Cue => ({ kind: 'voice', src, ...(delay_ms ? { delay_ms } : {}) })
  const music: Cue = { kind: 'music', src: 'M' }

  // p0: music and line A; reveal step 1 has line B after a pause.   p1: line C.   p2: no narration.
  const narrated = (duck_level?: number): Bundle => ({
    ...bundle,
    ...(duck_level !== undefined ? { audio: { duck_level } } : {}),
    chapters: [
      {
        id: 'one',
        title: 'One',
        content_hash: 'h',
        pages: [
          { blocks: [block('a', [music, voice('A')]), block('b', [voice('B', 500)], 1)] },
          { blocks: [block('c', [voice('C')])] },
          { blocks: [block('d', [])] },
        ],
      },
    ],
  })
  const at = (page: number): ReaderState => ({ position: { chapter: 0, page }, revealed: 0, direction: 'forward' })

  let engine: FakeEngine
  let enabled: boolean

  beforeEach(() => {
    vi.useFakeTimers()
    engine = new FakeEngine()
    enabled = true
  })
  afterEach(() => vi.useRealTimers())

  const director = (book = narrated()) => new AudioDirector(book, engine, { narrationEnabled: () => enabled })

  it('dips music while narration is heard, holding through pauses between lines', () => {
    const d = director()
    d.update(at(0), 'enter')
    expect(engine.ducks).toEqual(['0.35 300'])
    d.update({ ...at(0), revealed: 1 }, 'reveal')
    engine.end() // A ends; B waits out its pause: still ducked
    vi.advanceTimersByTime(500)
    expect(engine.ducks).toEqual(['0.35 300'])
    engine.end() // B ends; nothing queued
    expect(engine.ducks).toEqual(['0.35 300', '1 800'])
  })

  it('stays down across a page turn onto more narration, and comes back after one without', () => {
    const d = director()
    d.update(at(0), 'enter')
    d.update(at(1), 'turn')
    expect(engine.ducks, 'no dip and rise between pages').toEqual(['0.35 300'])
    d.update(at(2), 'turn')
    expect(engine.ducks).toEqual(['0.35 300', '1 800'])
  })

  it('never ducks for narration that is off, and releases when the reader mutes it mid-line', async () => {
    enabled = false
    const d = director()
    d.update(at(0), 'enter')
    await vi.advanceTimersByTimeAsync(5000)
    expect(engine.ducks).toEqual([])

    enabled = true
    d.update(at(1), 'turn')
    expect(engine.ducks).toEqual(['0.35 300'])
    enabled = false
    d.narrationSettingsChanged()
    expect(engine.ducks).toEqual(['0.35 300', '1 800'])
  })

  it('follows the book: its own level, or none at 1', () => {
    director(narrated(0.6)).update(at(0), 'enter')
    expect(engine.ducks).toEqual(['0.6 300'])
    engine.ducks = []
    director(narrated(1)).update(at(1), 'enter')
    expect(engine.ducks).toEqual([])
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
