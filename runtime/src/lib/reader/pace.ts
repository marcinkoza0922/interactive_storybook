import type { Bundle } from '../bundle/types'
import type { Position } from './navigation'

/** Words read and the time spent reading them, from pages read through. */
export interface Pace {
  words: number
  ms: number
}

/** The pace assumed before the reader's own is known. */
export const DEFAULT_WPM = 230
/** The default pace stands in for this many words until that many have been measured. */
const PRIOR_WORDS = 1000
/** Pages read faster than this were skipped; slower, left open while the reader was away. */
const PLAUSIBLE_WPM = { min: 60, max: 1000 }
/** Beyond this many words, older reading fades so the pace follows the reader's current one. */
const MEMORY_WORDS = 5000
const MINUTE_MS = 60_000

export const emptyPace = (): Pace => ({ words: 0, ms: 0 })

/** The reader's words per minute: the default, moving toward what's been measured. */
export function wordsPerMinute(pace: Pace): number {
  const priorWords = Math.max(0, PRIOR_WORDS - pace.words)
  const priorMs = (priorWords / DEFAULT_WPM) * MINUTE_MS
  return ((priorWords + pace.words) / (priorMs + pace.ms)) * MINUTE_MS
}

/** Add a page read through in `ms`, unless it was read implausibly fast or slow. */
export function withPage(pace: Pace, words: number, ms: number): Pace {
  if (words <= 0 || ms <= 0) return pace
  const wpm = (words / ms) * MINUTE_MS
  if (wpm < PLAUSIBLE_WPM.min || wpm > PLAUSIBLE_WPM.max) return pace
  const total = { words: pace.words + words, ms: pace.ms + ms }
  const keep = Math.min(1, MEMORY_WORDS / total.words)
  return { words: total.words * keep, ms: total.ms * keep }
}

/** Minutes to read from the start of this page to the end of its chapter. */
export function minutesLeft(bundle: Bundle, position: Position, pace: Pace): number {
  const words = bundle.chapters[position.chapter].pages.slice(position.page).reduce((sum, p) => sum + p.words, 0)
  return words / wordsPerMinute(pace)
}

export function timeLeftLabel(minutes: number): string {
  const rounded = Math.round(minutes)
  if (rounded < 1) return 'Less than a minute left in this chapter'
  return `About ${rounded} minute${rounded === 1 ? '' : 's'} left in this chapter`
}

/** The status bar's compact form: `9 min left`, `<1 min left`. */
export function shortTimeLeft(minutes: number): string {
  const rounded = Math.round(minutes)
  return rounded < 1 ? '<1 min' : `${rounded} min`
}

/** Time spent on the current page, not counting time paused (menu open, window hidden). */
export class PageClock {
  private started = 0
  private pausedMs = 0
  private pausedAt: number | null = null

  constructor(private now: () => number = () => performance.now()) {}

  restart(): void {
    this.started = this.now()
    this.pausedMs = 0
    if (this.pausedAt !== null) this.pausedAt = this.started
  }

  setPaused(paused: boolean): void {
    if (paused && this.pausedAt === null) this.pausedAt = this.now()
    if (!paused && this.pausedAt !== null) {
      this.pausedMs += this.now() - this.pausedAt
      this.pausedAt = null
    }
  }

  elapsed(): number {
    const pausing = this.pausedAt === null ? 0 : this.now() - this.pausedAt
    return this.now() - this.started - this.pausedMs - pausing
  }
}
