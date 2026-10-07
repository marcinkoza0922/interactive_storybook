import { describe, expect, it } from 'vitest'
import type { Bundle, Reference } from '../bundle/types'
import { pageReferences, visibleEntry } from './gating'

const chapters = ['one', 'two', 'three'].map((id) => ({ id, title: id, content_hash: 'h', pages: [{ blocks: [], words: 0 }] }))

const witch: Reference = {
  id: 'elara',
  sections: [
    { from: 'one', title: 'The Witch of Varn', html: 'A healer.', image: { src: 'witch.png', alt: 'A woman' } },
    { from: 'two', mode: 'append', html: 'Keeps ravens.' },
    { from: 'three', title: 'Elara', html: 'The exiled queen.' },
  ],
}

const bundle: Bundle = {
  bundle_schema_version: 2,
  book: { id: 't', title: 'T', author: 'A', language: 'en' },
  chapters,
  references: [
    witch,
    { id: 'late', sections: [{ from: 'three', title: 'Late', html: 'Only later.' }] },
    { id: 'broken', sections: [{ from: 'missing', title: 'Broken', html: 'Never shown.' }] },
  ],
}

describe('visibleEntry', () => {
  it('shows only the first section in the first chapter', () => {
    expect(visibleEntry(bundle, witch, 0)).toEqual({
      id: 'elara',
      title: 'The Witch of Varn',
      sections: ['A healer.'],
      image: { src: 'witch.png', alt: 'A woman' },
    })
  })

  it('appends, carrying the title and image forward', () => {
    expect(visibleEntry(bundle, witch, 1)).toMatchObject({
      title: 'The Witch of Varn',
      sections: ['A healer.', 'Keeps ravens.'],
      image: { src: 'witch.png' },
    })
  })

  it('replaces everything, including the gated name and the image', () => {
    expect(visibleEntry(bundle, witch, 2)).toEqual({ id: 'elara', title: 'Elara', sections: ['The exiled queen.'] })
  })

  it('walks sections in chapter order regardless of file order', () => {
    const shuffled = { ...witch, sections: [witch.sections[2], witch.sections[0], witch.sections[1]] }
    expect(visibleEntry(bundle, shuffled, 2)?.title).toBe('Elara')
  })

  it('hides references with nothing unlocked, an unknown chapter, or no title', () => {
    expect(visibleEntry(bundle, bundle.references![1], 1)).toBeNull()
    expect(visibleEntry(bundle, bundle.references![2], 2)).toBeNull()
    expect(visibleEntry(bundle, { id: 'x', sections: [{ from: 'one', html: 'Untitled.' }] }, 2)).toBeNull()
  })
})

describe('pageReferences', () => {
  it('lists unlocked references on the page in order and skips the rest', () => {
    const page = { blocks: [], words: 0, references: ['late', 'elara', 'unknown'] }
    expect(pageReferences(bundle, page, 0).map((e) => e.id)).toEqual(['elara'])
    expect(pageReferences(bundle, page, 2).map((e) => e.id)).toEqual(['late', 'elara'])
  })
})
