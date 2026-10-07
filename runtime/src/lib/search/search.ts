import type { Bundle } from '../bundle/types'
import { visibleTextRoot } from '../highlights/anchor'

/** Where a match lies: a block's visible text, or a footnote's. */
export type SearchTarget = { kind: 'block'; blockId: string } | { kind: 'footnote'; number: number }

export interface SearchMatch {
  chapter: number
  page: number
  target: SearchTarget
  /** Character offsets of the match in the target's text, as `rangeIn` takes them. */
  start: number
  end: number
  snippet: { before: string; match: string; after: string }
}

/** A piece of searchable text, folded for matching, with a way back to the original. */
interface Entry {
  chapter: number
  page: number
  target: SearchTarget
  text: string
  folded: string
  /** Offset in `text` of each code unit of `folded`, then the end of `text`. */
  origin: number[]
}

/** Typographic punctuation a reader is unlikely to type. */
const PUNCTUATION: Record<string, string> = { '‘': "'", '’': "'", 'ʼ': "'", '“': '"', '”': '"' }
const WORD = /[\p{L}\p{N}]/u
const SNIPPET_CHARS = 40

/** Case- and accent-insensitive form of one character; whitespace becomes a single space. */
function foldChar(char: string): string {
  if (/\s/.test(char)) return ' '
  return PUNCTUATION[char] ?? char.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase()
}

class Folder {
  folded = ''
  origin: number[] = []

  /** Fold `data`, which starts at `offset` in the original text. */
  add(data: string, offset: number): void {
    for (const char of data) {
      const folded = foldChar(char)
      // Whitespace runs collapse, so phrases match across line breaks.
      if (!(folded === ' ' && (this.folded === '' || this.folded.endsWith(' ')))) {
        this.folded += folded
        for (let i = 0; i < folded.length; i++) this.origin.push(offset)
      }
      offset += char.length
    }
  }

  /** A word break in place of text that isn't searched, such as a footnote marker. */
  addBreak(offset: number): void {
    if (this.folded === '' || this.folded.endsWith(' ')) return
    this.folded += ' '
    this.origin.push(offset)
  }
}

/** Fold a query the same way as the text. */
export function foldQuery(query: string): string {
  const folder = new Folder()
  folder.add(query, 0)
  return folder.folded.trim()
}

function entry(root: Element, chapter: number, page: number, target: SearchTarget): Entry {
  const folder = new Folder()
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT)
  let offset = 0
  while (walker.nextNode()) {
    const node = walker.currentNode as Text
    // A footnote marker's number isn't part of the word it follows.
    if (node.parentElement?.closest('[data-tome-footnote]')) folder.addBreak(offset)
    else folder.add(node.data, offset)
    offset += node.data.length
  }
  folder.origin.push(offset)
  return { chapter, page, target, text: root.textContent ?? '', folded: folder.folded, origin: folder.origin }
}

function parse(html: string): HTMLElement {
  const container = document.createElement('div')
  container.innerHTML = html
  return container
}

/**
 * Every block's visible text and every footnote, in reading order. A footnote that travels
 * with several pages is indexed on the first.
 */
function buildIndex(bundle: Bundle): Entry[] {
  const entries: Entry[] = []
  bundle.chapters.forEach((chapter, c) => {
    const footnotes = new Set<number>()
    chapter.pages.forEach((page, p) => {
      for (const block of page.blocks) {
        entries.push(entry(visibleTextRoot(parse(block.html)), c, p, { kind: 'block', blockId: block.id }))
      }
      for (const footnote of page.footnotes ?? []) {
        if (footnotes.has(footnote.number)) continue
        footnotes.add(footnote.number)
        entries.push(entry(parse(footnote.html), c, p, { kind: 'footnote', number: footnote.number }))
      }
    })
  })
  return entries
}

const indexes = new WeakMap<Bundle, Entry[]>()

/** The search index, built from the bundle's text the first time it's needed. */
function indexFor(bundle: Bundle): Entry[] {
  let index = indexes.get(bundle)
  if (!index) {
    index = buildIndex(bundle)
    indexes.set(bundle, index)
  }
  return index
}

const isWord = (char: string | undefined) => char !== undefined && WORD.test(char)

/** Whether `folded[start, end)` is whole words: no letters or digits run on past either end. */
function wholeWords(folded: string, start: number, end: number): boolean {
  const runsOnBefore = isWord(folded[start]) && isWord(folded[start - 1])
  const runsOnAfter = isWord(folded[end - 1]) && isWord(folded[end])
  return !runsOnBefore && !runsOnAfter
}

function snippet(text: string, start: number, end: number): SearchMatch['snippet'] {
  const tidy = (s: string) => s.replace(/\s+/g, ' ')
  let from = Math.max(0, start - SNIPPET_CHARS)
  let to = Math.min(text.length, end + SNIPPET_CHARS)
  // Cut at word breaks rather than mid-word.
  if (from > 0) from = text.indexOf(' ', from) + 1 || start
  if (to < text.length) to = Math.max(end, text.lastIndexOf(' ', to))
  if (from > start) from = start
  return {
    before: (from > 0 ? '…' : '') + tidy(text.slice(from, start)).trimStart(),
    match: tidy(text.slice(start, end)),
    after: tidy(text.slice(end, to)).trimEnd() + (to < text.length ? '…' : ''),
  }
}

/**
 * Find whole words or phrases in the text of chapters up to `lastChapter`, in reading order.
 * Later chapters are never searched, so results can't spoil them. Stops after `limit` matches.
 */
export function search(bundle: Bundle, query: string, lastChapter: number, limit = 200): SearchMatch[] {
  const needle = foldQuery(query)
  if (!needle) return []

  const matches: SearchMatch[] = []
  for (const { chapter, page, target, text, folded, origin } of indexFor(bundle)) {
    if (chapter > lastChapter) break
    for (let at = folded.indexOf(needle); at !== -1; at = folded.indexOf(needle, at + 1)) {
      if (!wholeWords(folded, at, at + needle.length)) continue
      const start = origin[at]
      const end = origin[at + needle.length]
      matches.push({ chapter, page, target, start, end, snippet: snippet(text, start, end) })
      if (matches.length >= limit) return matches
    }
  }
  return matches
}
