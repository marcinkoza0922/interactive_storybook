import type { Bundle, ImageRef, Page, Reference } from '../bundle/types'

/** A reference as the reader may currently see it. */
export interface VisibleEntry {
  id: string
  title: string
  /** HTML of each visible section, in order. */
  sections: string[]
  image?: ImageRef
}

/**
 * Compute what the reader knows about a reference, given the furthest chapter they have
 * reached. Unlocked sections are walked in chapter order: `replace` starts over, `append`
 * adds. Title and image carry forward until a later unlocked section changes them.
 * Returns null while nothing is unlocked, or if no unlocked section names the reference.
 */
export function visibleEntry(bundle: Bundle, reference: Reference, furthestChapter: number): VisibleEntry | null {
  const order = new Map(bundle.chapters.map((c, i) => [c.id, i]))
  // Sections gated on an unknown chapter never unlock: hiding is the safe failure.
  const unlocked = reference.sections
    .map((section) => ({ section, chapter: order.get(section.from) ?? Infinity }))
    .filter(({ chapter }) => chapter <= furthestChapter)
    .sort((a, b) => a.chapter - b.chapter)

  let title: string | undefined
  let image: ImageRef | undefined
  let sections: string[] = []

  for (const { section } of unlocked) {
    if ((section.mode ?? 'replace') === 'replace') {
      sections = []
      image = undefined
    }
    sections.push(section.html)
    title = section.title ?? title
    image = section.image ?? image
  }

  if (sections.length === 0 || !title) return null
  return { id: reference.id, title, sections, ...(image ? { image } : {}) }
}

/** The visible references mentioned on a page, in order of first mention. */
export function pageReferences(bundle: Bundle, page: Page, furthestChapter: number): VisibleEntry[] {
  const byId = new Map((bundle.references ?? []).map((r) => [r.id, r]))
  return (page.references ?? []).flatMap((id) => {
    const reference = byId.get(id)
    const entry = reference && visibleEntry(bundle, reference, furthestChapter)
    return entry ? [entry] : []
  })
}

export function referenceEntry(bundle: Bundle, id: string, furthestChapter: number): VisibleEntry | null {
  const reference = bundle.references?.find((r) => r.id === id)
  return reference ? visibleEntry(bundle, reference, furthestChapter) : null
}
