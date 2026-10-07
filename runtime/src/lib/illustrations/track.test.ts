import { describe, expect, it } from 'vitest'
import type { Bundle, Page } from '../bundle/types'
import { introducesIllustration, trackStates } from './track'

const plate = (src: string) => ({ src, alt: src })
const page = (illustration?: Page['illustration']): Page => ({ blocks: [], words: 0, ...(illustration !== undefined ? { illustration } : {}) })

// ch1: [no art, A, (carry)]   ch2: [(carry), A again, null, B]
const bundle: Bundle = {
  bundle_schema_version: 2,
  book: { id: 't', title: 'T', author: 'A', language: 'en' },
  chapters: [
    { id: 'one', title: 'One', content_hash: 'h', pages: [page(), page(plate('A')), page()] },
    { id: 'two', title: 'Two', content_hash: 'h', pages: [page(), page(plate('A')), page(null), page(plate('B'))] },
  ],
}

const states = trackStates(bundle)
const srcs = states.map((pages) => pages.map((p) => p?.src ?? '-'))

describe('trackStates', () => {
  it('carries the illustration across pages and chapters until changed or cleared', () => {
    expect(srcs).toEqual([
      ['-', 'A', 'A'],
      ['A', 'A', '-', 'B'],
    ])
  })
})

describe('introducesIllustration', () => {
  it('is true only where the track changes to a new image', () => {
    const introduced = bundle.chapters.map((c, chapter) => c.pages.map((_, page) => introducesIllustration(states, { chapter, page })))
    expect(introduced).toEqual([
      [false, true, false],
      [false, false, false, true],
    ])
  })
})
