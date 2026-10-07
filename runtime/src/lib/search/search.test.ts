// @vitest-environment happy-dom
import { describe, expect, it } from 'vitest'
import type { Bundle, Page } from '../bundle/types'
import { foldQuery, search } from './search'

const page = (...html: string[]): Page => ({ blocks: html.map((h, i) => ({ id: `b${Math.random()}-${i}`, html: h })), words: 0 })

function book(...chapters: Page[][]): Bundle {
  return {
    bundle_schema_version: 2,
    book: { id: 't', title: 'T', author: 'A', language: 'en' },
    chapters: chapters.map((pages, i) => ({ id: `c${i}`, title: `Chapter ${i + 1}`, content_hash: 'h', pages })),
  }
}

const found = (bundle: Bundle, query: string, lastChapter = bundle.chapters.length - 1) =>
  search(bundle, query, lastChapter).map((m) => m.snippet.match)

describe('foldQuery', () => {
  it('ignores case, accents, curly quotes and extra spaces', () => {
    expect(foldQuery('  Café  “Noël”  ')).toBe('cafe "noel"')
  })
})

describe('search', () => {
  it('matches case- and accent-insensitively, returning the original text', () => {
    const bundle = book([page('<p>The CAFÉ on the quay.</p>')])
    expect(found(bundle, 'cafe')).toEqual(['CAFÉ'])
  })

  it('matches whole words only', () => {
    const bundle = book([page('<p>The bell, the bells, a doorbell.</p>')])
    expect(found(bundle, 'bell')).toEqual(['bell'])
  })

  it('matches phrases across markup and line breaks', () => {
    const bundle = book([page('<p>the <em>uneven</em>\n  bell rang</p>')])
    const [match] = search(bundle, 'uneven bell', 0)
    expect(match.snippet.match).toBe('uneven bell')
    expect(match.start).toBe(4)
    expect(match.end).toBe(17)
  })

  it('matches straight apostrophes against curly ones', () => {
    expect(found(book([page('<p>She didn’t look back.</p>')]), "didn't")).toEqual(['didn’t'])
  })

  it('never searches past the last chapter allowed', () => {
    const bundle = book([page('<p>A lantern.</p>')], [page('<p>The lantern was a lie.</p>')])
    expect(search(bundle, 'lantern', 0).map((m) => m.chapter)).toEqual([0])
    expect(search(bundle, 'lantern', 1).map((m) => m.chapter)).toEqual([0, 1])
  })

  it('searches footnotes once, on the first page that carries them', () => {
    const note = { number: 1, html: '<p>A tidal bell.</p>' }
    const bundle = book([
      { ...page('<p>See<sup data-tome-footnote="1">1</sup>.</p>'), footnotes: [note] },
      { ...page('<p>Again<sup data-tome-footnote="1">1</sup>.</p>'), footnotes: [note] },
    ])
    expect(search(bundle, 'tidal', 0)).toMatchObject([{ page: 0, target: { kind: 'footnote', number: 1 } }])
  })

  it('keeps footnote markers from joining the words around them', () => {
    const bundle = book([page('<p>The bell<sup data-tome-footnote="2">2</sup> rang.</p>')])
    const [match] = search(bundle, 'bell rang', 0)
    expect(match.snippet.match).toBe('bell2 rang')
    expect(found(bundle, 'bell')).toEqual(['bell'])
  })

  it('offsets into the animated copy of a block, which is what the reader sees', () => {
    const html = '<div class="tome-sr-only"><p>Hush now.</p></div><div class="tome-visual"><p>Hush now.</p></div>'
    const [match] = search(book([page(html)]), 'now', 0)
    expect(match).toMatchObject({ start: 5, end: 8 })
    expect(found(book([page(html)]), 'now')).toHaveLength(1)
  })

  it('gives a snippet around the match, cut at word breaks', () => {
    const text = 'One two three four five six seven eight nine ten eleven twelve thirteen fourteen needle fifteen sixteen'
    const [match] = search(book([page(`<p>${text}</p>`)]), 'needle', 0)
    expect(match.snippet.before.startsWith('…')).toBe(true)
    expect(match.snippet.before).toMatch(/ thirteen fourteen $/)
    expect(match.snippet.after).toBe(' fifteen sixteen')
  })

  it('finds nothing for an empty query, and stops at the limit', () => {
    const bundle = book([page('<p>a a a a a</p>')])
    expect(search(bundle, '   ', 0)).toEqual([])
    expect(search(bundle, 'a', 0, 3)).toHaveLength(3)
  })
})
