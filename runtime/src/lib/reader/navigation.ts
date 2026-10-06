import type { Bundle, Page } from '../bundle/types'

/**
 * How the reader arrived at the current page. Drives direction-dependent behaviour:
 * moving forward plays reveals (and, later, sound effects); moving backward shows the
 * page fully revealed. Jumps and resumes to already-seen pages count as backward.
 */
export type Direction = 'forward' | 'backward'

export interface Position {
  chapter: number
  page: number
}

export interface ReaderState {
  position: Position
  /** Number of reveal steps shown on the current page (0 = only unrevealed blocks). */
  revealed: number
  direction: Direction
}

export function pageAt(bundle: Bundle, position: Position): Page {
  return bundle.chapters[position.chapter].pages[position.page]
}

/** Number of reveal steps on a page; 0 if it has no reveal blocks. */
export function stepCount(page: Page): number {
  return page.blocks.reduce((max, block) => Math.max(max, block.reveal?.step ?? 0), 0)
}

export function isFullyRevealed(bundle: Bundle, state: ReaderState): boolean {
  return state.revealed >= stepCount(pageAt(bundle, state.position))
}

function nextPosition(bundle: Bundle, { chapter, page }: Position): Position | null {
  if (page + 1 < bundle.chapters[chapter].pages.length) return { chapter, page: page + 1 }
  if (chapter + 1 < bundle.chapters.length) return { chapter: chapter + 1, page: 0 }
  return null
}

function previousPosition(bundle: Bundle, { chapter, page }: Position): Position | null {
  if (page > 0) return { chapter, page: page - 1 }
  if (chapter > 0) return { chapter: chapter - 1, page: bundle.chapters[chapter - 1].pages.length - 1 }
  return null
}

/** The single advance action: reveal the next step, or turn the page once fully revealed. */
export function advance(bundle: Bundle, state: ReaderState): ReaderState | null {
  if (!isFullyRevealed(bundle, state)) {
    return { ...state, revealed: state.revealed + 1 }
  }
  const next = nextPosition(bundle, state.position)
  return next && { position: next, revealed: 0, direction: 'forward' }
}

/** Go to the previous page, shown fully revealed. */
export function back(bundle: Bundle, state: ReaderState): ReaderState | null {
  const previous = previousPosition(bundle, state.position)
  return previous && arriveBackward(bundle, previous)
}

/** Arrive at an already-seen page (back, resume, or a jump to a seen page). */
export function arriveBackward(bundle: Bundle, position: Position): ReaderState {
  return { position, revealed: stepCount(pageAt(bundle, position)), direction: 'backward' }
}
