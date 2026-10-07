import { describe, expect, it } from 'vitest'
import type { Block, Bundle, Page } from '../bundle/types'
import { advance, arriveBackward, back, type ReaderState } from './navigation'

const plain = (id: string): Block => ({ id, html: `<p>${id}</p>` })
const revealed = (id: string, step: number): Block => ({ ...plain(id), reveal: { step, effect: 'fade' } })
const page = (...blocks: Block[]): Page => ({ blocks, words: 0 })

// ch1: [plain page, page with 2 reveal steps]; ch2: [plain page]
const bundle: Bundle = {
  bundle_schema_version: 2,
  book: { id: 'test', title: 'Test', author: 'A', language: 'en' },
  chapters: [
    { id: 'one', title: 'One', content_hash: 'h1', pages: [page(plain('a')), page(plain('b'), revealed('c', 1), revealed('d', 2))] },
    { id: 'two', title: 'Two', content_hash: 'h2', pages: [page(plain('e'))] },
  ],
}

const at = (chapter: number, page: number, revealed = 0): ReaderState => ({
  position: { chapter, page },
  revealed,
  direction: 'forward',
})

describe('advance', () => {
  it('turns a page with no reveals', () => {
    expect(advance(bundle, at(0, 0))).toEqual(at(0, 1))
  })

  it('reveals one step at a time before turning the page', () => {
    expect(advance(bundle, at(0, 1, 0))).toEqual(at(0, 1, 1))
    expect(advance(bundle, at(0, 1, 1))).toEqual(at(0, 1, 2))
    expect(advance(bundle, at(0, 1, 2))).toEqual(at(1, 0))
  })

  it('stops at the end of the book', () => {
    expect(advance(bundle, at(1, 0))).toBeNull()
  })
})

describe('back', () => {
  it('crosses chapter boundaries and shows the page fully revealed', () => {
    expect(back(bundle, at(1, 0))).toEqual({ position: { chapter: 0, page: 1 }, revealed: 2, direction: 'backward' })
  })

  it('stops at the start of the book', () => {
    expect(back(bundle, at(0, 0))).toBeNull()
  })

  it('then advancing from a fully revealed page turns it', () => {
    expect(advance(bundle, arriveBackward(bundle, { chapter: 0, page: 1 }))).toEqual(at(1, 0))
  })
})
