import type { Block, Bundle, Cue } from '../bundle/types'
import { pageAt, type Position, type ReaderState } from '../reader/navigation'
import type { AudioEngine, VoiceHandle } from './engine'
import { applyCue, pageEndStates, sameVoice, SILENCE, type AudioState } from './state'

export const DEFAULT_RESTORE_DELAY_MS = 3000
export const DEFAULT_MUSIC_FADE_IN_MS = 1000
export const DEFAULT_MUSIC_FADE_OUT_MS = 2000
export const DEFAULT_AMBIENT_FADE_MS = 1500
/** How fast a line of narration fades when a page turn cuts it off. */
export const NARRATION_CUT_MS = 150
/** Music and ambience volume under narration, unless the book sets its own. */
export const DEFAULT_DUCK_LEVEL = 0.35
/** Ducking dips quickly when a line starts and recovers gently when narration ends. */
export const DUCK_ATTACK_MS = 300
export const DUCK_RELEASE_MS = 800

export interface DirectorOptions {
  /**
   * False when the reader can't hear narration (muted or at zero). Lines then pass silently,
   * each taking as long as its recording, so auto mode keeps the narration's pace.
   */
  narrationEnabled?: () => boolean
  /** Called when narration starts or stops (a line playing, silent, waiting out its delay, or queued). */
  onNarrationChange?: (narrating: boolean) => void
}

type NarrationCue = Extract<Cue, { kind: 'voice' }>

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
 *
 * Narration (voice cues) plays one line at a time, strictly in order: lines revealed on the
 * same page queue behind the one playing, and a line's delay is a pause before it once its
 * turn comes. Any page turn cuts the current line off and drops the queue. Like sound
 * effects, narration only plays moving forward.
 */
export class AudioDirector {
  private current: AudioState = SILENCE
  private timers = new Set<ReturnType<typeof setTimeout>>()
  private endStates: AudioState[][]
  private restoreDelayMs: number

  /** The line in progress, whether waiting out its delay, playing, or passing silently. */
  private narration: VoiceHandle | null = null
  private narrationQueue: NarrationCue[] = []
  private wasNarrating = false
  private duckLevel: number
  private ducked = false

  constructor(
    private bundle: Bundle,
    private engine: AudioEngine,
    private options: DirectorOptions = {},
  ) {
    this.endStates = pageEndStates(bundle)
    this.restoreDelayMs = bundle.audio?.restore_delay_ms ?? DEFAULT_RESTORE_DELAY_MS
    this.duckLevel = bundle.audio?.duck_level ?? DEFAULT_DUCK_LEVEL
  }

  /** Whether a line of narration is in progress or queued, heard or not. */
  get narrating(): boolean {
    return this.narration !== null || this.narrationQueue.length > 0
  }

  /**
   * Cut narration off and drop the queue. `release` brings ducked music back up; a page turn
   * holds it down until it knows whether the new page starts narrating too.
   */
  stopNarration(release = true): void {
    this.narrationQueue = []
    this.narration?.stop(NARRATION_CUT_MS)
    this.narration = null
    if (release) this.setDucked(false)
    this.notifyNarration()
  }

  /** Call when the reader's narration settings change: muted narration doesn't duck. */
  narrationSettingsChanged(): void {
    if (!this.narrationEnabled()) this.setDucked(false)
  }

  update(reader: ReaderState, change: Change): void {
    const page = pageAt(this.bundle, reader.position)

    if (change === 'reveal') {
      this.fire(this.current, page.blocks.filter((b) => b.reveal?.step === reader.revealed))
    } else {
      // Delayed cues belong to the page being left. Their effect on music and ambience
      // is already captured in the page states, so dropping them loses nothing lasting.
      // A page turn also cuts off the narration of the page being left.
      this.cancelPending()
      this.stopNarration(false)

      if (reader.direction === 'forward') {
        const visible = page.blocks.filter((b) => (b.reveal?.step ?? 0) <= reader.revealed)
        this.fire(this.stateBefore(reader.position), visible)
      } else if (change === 'enter') {
        this.reconcile(this.stateAtEnd(reader.position))
      } else {
        const target = this.stateAtEnd(reader.position)
        this.schedule(() => this.reconcile(target), this.restoreDelayMs)
      }
      // Music comes back up unless the new page started narrating.
      if (!this.narrating) this.setDucked(false)
    }

    this.preloadAround(reader.position)
  }

  /** Fade everything out, e.g. when leaving the reader. */
  stop(): void {
    this.cancelPending()
    this.stopNarration()
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
      // Narration keeps its own order and timing, delays included.
      if (cue.kind === 'voice') {
        this.narrate(cue)
      } else if (cue.delay_ms && cue.delay_ms > 0) {
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
    if (cue.kind === 'voice') this.narrate(cue)
    else if (cue.kind === 'sfx') this.engine.playSfx(cue.src, cue.volume ?? 1)
    else this.reconcile(applyCue(this.current, cue), { [voiceKey(cue)]: fadeFor(cue) })
  }

  private narrationEnabled(): boolean {
    return this.options.narrationEnabled?.() ?? true
  }

  /** Queue a line, starting it at once if nothing is in progress. */
  private narrate(cue: NarrationCue): void {
    this.narrationQueue.push(cue)
    if (!this.narration) this.nextNarration()
    else this.notifyNarration()
  }

  /**
   * Start the next line: wait out its delay, then play it, or pass it silently for as long as
   * it lasts if the reader can't hear narration. Each phase replaces `this.narration`, so a
   * page turn cancels whichever phase is in progress.
   */
  private nextNarration(): void {
    const cue = this.narrationQueue.shift()
    if (!cue) {
      this.narration = null
      this.setDucked(false)
      return this.notifyNarration()
    }

    const line: VoiceHandle = { stop: () => phase.stop(NARRATION_CUT_MS) }
    let phase: VoiceHandle
    const current = () => this.narration === line
    const finished = () => current() && this.nextNarration()

    const begin = () => {
      if (!current()) return
      if (this.narrationEnabled()) {
        this.setDucked(true)
        phase = this.engine.playVoice(cue.src, cue.volume ?? 1, finished)
      } else {
        let timer: ReturnType<typeof setTimeout> | undefined
        let cancelled = false
        phase = {
          stop: () => {
            cancelled = true
            clearTimeout(timer)
          },
        }
        this.engine.voiceDuration(cue.src).then((ms) => {
          if (!cancelled) timer = setTimeout(finished, ms)
        })
      }
    }

    this.narration = line
    if (cue.delay_ms && cue.delay_ms > 0) {
      const timer = setTimeout(begin, cue.delay_ms)
      phase = { stop: () => clearTimeout(timer) }
    } else {
      phase = { stop: () => {} }
      begin()
    }
    this.notifyNarration()
  }

  private setDucked(ducked: boolean): void {
    if (ducked === this.ducked || this.duckLevel >= 1) return
    this.ducked = ducked
    this.engine.setDucking(ducked ? this.duckLevel : 1, ducked ? DUCK_ATTACK_MS : DUCK_RELEASE_MS)
  }

  private notifyNarration(): void {
    const narrating = this.narrating
    if (narrating !== this.wasNarrating) {
      this.wasNarrating = narrating
      this.options.onNarrationChange?.(narrating)
    }
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
  if (cue.kind === 'sfx' || cue.kind === 'voice') return 0
  if (cue.fade_ms !== undefined) return cue.fade_ms
  if (cue.kind === 'music') return DEFAULT_MUSIC_FADE_IN_MS
  if (cue.kind === 'music_stop') return DEFAULT_MUSIC_FADE_OUT_MS
  return DEFAULT_AMBIENT_FADE_MS
}
