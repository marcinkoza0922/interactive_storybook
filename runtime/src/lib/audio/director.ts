import type { Block, Bundle, Cue } from '../bundle/types'
import { pageAt, type Position, type ReaderState } from '../reader/navigation'
import type { AudioEngine } from './engine'
import { applyCue, pageEndStates, sameVoice, SILENCE, type AudioState } from './state'

export const DEFAULT_RESTORE_DELAY_MS = 3000
export const DEFAULT_MUSIC_FADE_IN_MS = 1000
export const DEFAULT_MUSIC_FADE_OUT_MS = 2000
export const DEFAULT_AMBIENT_FADE_MS = 1500

/**
 * - `enter`: reading starts (begin or resume)
 * - `turn`: the reader arrived on another page
 * - `reveal`: a reveal step on the current page appeared
 */
export type Change = 'enter' | 'turn' | 'reveal'

/** Per-voice fade overrides, keyed by `music` or `ambient:<id>`. */
type Fades = Record<string, number>

/**
 * Decides what the reader hears, following the spec's direction-dependent rules:
 * moving forward fires the cues of newly visible blocks (sound effects included);
 * moving back plays no sound effects and restores the page's music and ambience
 * after a delay, so glancing back at a detail doesn't cut in on the current music.
 */
export class AudioDirector {
  private current: AudioState = SILENCE
  private timers = new Set<ReturnType<typeof setTimeout>>()
  private endStates: AudioState[][]
  private restoreDelayMs: number

  constructor(
    private bundle: Bundle,
    private engine: AudioEngine,
  ) {
    this.endStates = pageEndStates(bundle)
    this.restoreDelayMs = bundle.audio?.restore_delay_ms ?? DEFAULT_RESTORE_DELAY_MS
  }

  update(reader: ReaderState, change: Change): void {
    const page = pageAt(this.bundle, reader.position)

    if (change === 'reveal') {
      this.fire(this.current, page.blocks.filter((b) => b.reveal?.step === reader.revealed))
    } else {
      // Delayed cues belong to the page being left. Their effect on music and ambience
      // is already captured in the page states, so dropping them loses nothing lasting.
      this.cancelPending()

      if (reader.direction === 'forward') {
        const visible = page.blocks.filter((b) => (b.reveal?.step ?? 0) <= reader.revealed)
        this.fire(this.stateBefore(reader.position), visible)
      } else if (change === 'enter') {
        this.reconcile(this.stateAtEnd(reader.position))
      } else {
        const target = this.stateAtEnd(reader.position)
        this.schedule(() => this.reconcile(target), this.restoreDelayMs)
      }
    }

    this.preloadAround(reader.position)
  }

  /** Fade everything out, e.g. when leaving the reader. */
  stop(): void {
    this.cancelPending()
    this.reconcile(SILENCE)
  }

  /**
   * Apply the cues of `blocks` on top of `base`. Immediate continuous cues are folded into a
   * single transition, so arriving on a page never fades to an intermediate state first.
   */
  private fire(base: AudioState, blocks: Block[]): void {
    let target = base
    const fades: Fades = {}

    for (const cue of blocks.flatMap((b) => b.cues ?? [])) {
      if (cue.delay_ms && cue.delay_ms > 0) {
        this.schedule(() => this.play(cue), cue.delay_ms)
      } else if (cue.kind === 'sfx') {
        this.engine.playSfx(cue.src, cue.volume ?? 1)
      } else {
        target = applyCue(target, cue)
        fades[voiceKey(cue)] = fadeFor(cue)
      }
    }
    this.reconcile(target, fades)
  }

  private play(cue: Cue): void {
    if (cue.kind === 'sfx') this.engine.playSfx(cue.src, cue.volume ?? 1)
    else this.reconcile(applyCue(this.current, cue), { [voiceKey(cue)]: fadeFor(cue) })
  }

  /** Transition from what is sounding now to `target`, touching only voices that differ. */
  private reconcile(target: AudioState, fades: Fades = {}): void {
    if (!sameVoice(this.current.music, target.music)) {
      const fade = fades.music ?? (target.music ? DEFAULT_MUSIC_FADE_IN_MS : DEFAULT_MUSIC_FADE_OUT_MS)
      this.engine.setMusic(target.music, fade)
    }

    const ids = new Set([...Object.keys(this.current.ambient), ...Object.keys(target.ambient)])
    for (const id of ids) {
      if (!sameVoice(this.current.ambient[id], target.ambient[id])) {
        this.engine.setAmbient(id, target.ambient[id] ?? null, fades[`ambient:${id}`] ?? DEFAULT_AMBIENT_FADE_MS)
      }
    }

    this.current = target
  }

  private stateAtEnd({ chapter, page }: Position): AudioState {
    return this.endStates[chapter][page]
  }

  private stateBefore({ chapter, page }: Position): AudioState {
    if (page > 0) return this.endStates[chapter][page - 1]
    if (chapter > 0) return this.endStates[chapter - 1].at(-1)!
    return SILENCE
  }

  /** Fetch upcoming sounds so effects aren't late the first time they play. */
  private preloadAround({ chapter, page }: Position): void {
    const pages = [this.bundle.chapters[chapter].pages[page], this.bundle.chapters[chapter].pages[page + 1]]
    if (page + 1 >= this.bundle.chapters[chapter].pages.length) pages.push(this.bundle.chapters[chapter + 1]?.pages[0])

    const srcs = pages
      .flatMap((p) => p?.blocks ?? [])
      .flatMap((b) => b.cues ?? [])
      .flatMap((c) => (c.kind === 'sfx' || c.kind === 'ambient' ? [c.src] : []))
    if (srcs.length > 0) this.engine.preload(srcs)
  }

  private schedule(action: () => void, delayMs: number): void {
    const timer = setTimeout(() => {
      this.timers.delete(timer)
      action()
    }, delayMs)
    this.timers.add(timer)
  }

  private cancelPending(): void {
    for (const timer of this.timers) clearTimeout(timer)
    this.timers.clear()
  }
}

function voiceKey(cue: Cue): string {
  return cue.kind === 'ambient' || cue.kind === 'ambient_stop' ? `ambient:${cue.id}` : 'music'
}

function fadeFor(cue: Cue): number {
  if (cue.kind === 'sfx') return 0
  if (cue.fade_ms !== undefined) return cue.fade_ms
  if (cue.kind === 'music') return DEFAULT_MUSIC_FADE_IN_MS
  if (cue.kind === 'music_stop') return DEFAULT_MUSIC_FADE_OUT_MS
  return DEFAULT_AMBIENT_FADE_MS
}
