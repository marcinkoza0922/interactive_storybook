import type { Bundle, ImageRef } from '../bundle/types'
import type { Position } from '../reader/navigation'

export const DEFAULT_LINGER_MS = 3000

/** The track illustration on every page, indexed [chapter][page]. Carries across chapters. */
export function trackStates(bundle: Bundle): (ImageRef | null)[][] {
  let current: ImageRef | null = null
  return bundle.chapters.map((chapter) =>
    chapter.pages.map((page) => {
      if (page.illustration !== undefined) current = page.illustration
      return current
    }),
  )
}

/** Whether arriving on this page brings a different track illustration than the page before. */
export function introducesIllustration(states: (ImageRef | null)[][], { chapter, page }: Position): boolean {
  const current = states[chapter][page]
  const previous = page > 0 ? states[chapter][page - 1] : chapter > 0 ? states[chapter - 1].at(-1)! : null
  return current !== null && current.src !== previous?.src
}
