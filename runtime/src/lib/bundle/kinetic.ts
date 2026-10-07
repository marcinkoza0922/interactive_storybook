import type { Block } from './types'

/** Default typewriter pace when the author doesn't set a duration. */
export const TYPEWRITER_CHAR_MS = 35

/** Resting effects that animate each character. */
const PER_CHARACTER = new Set(['wave', 'tremble'])
const INLINE_TAGS = new Set(['SPAN', 'EM', 'STRONG', 'I', 'B', 'Q', 'MARK', 'SMALL', 'A', 'CITE'])

const segmenter = 'Segmenter' in Intl ? new Intl.Segmenter(undefined, { granularity: 'grapheme' }) : null

/** User-perceived characters, so accented letters and emoji aren't split apart. */
function graphemes(text: string): string[] {
  return segmenter ? Array.from(segmenter.segment(text), (s) => s.segment) : Array.from(text)
}

function textNodes(root: Node): Text[] {
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT)
  const nodes: Text[] = []
  while (walker.nextNode()) nodes.push(walker.currentNode as Text)
  return nodes
}

function span(className: string, text?: string): HTMLSpanElement {
  const element = document.createElement('span')
  element.className = className
  if (text !== undefined) element.textContent = text
  return element
}

/**
 * Wrap each word in an unbreakable span (so animated text still wraps between words only),
 * and optionally each character in its own span, numbered for phase offsets.
 */
function splitWords(root: Element, characters: boolean, counter: { index: number }): void {
  for (const node of textNodes(root)) {
    const fragment = document.createDocumentFragment()
    for (const part of node.data.split(/(\s+)/)) {
      if (!part) continue
      if (/^\s+$/.test(part)) {
        fragment.append(part)
        continue
      }
      const word = span('tome-word', characters ? undefined : part)
      if (characters) {
        for (const grapheme of graphemes(part)) {
          const char = span('tome-char', grapheme)
          char.style.setProperty('--tome-char-index', String(counter.index++))
          word.append(char)
        }
      }
      fragment.append(word)
    }
    node.replaceWith(fragment)
  }
}

/** Put every character (spaces included) in a numbered span; returns how many there are. */
function splitTypewriter(root: DocumentFragment): number {
  let index = 0
  for (const node of textNodes(root)) {
    // Whitespace between top-level elements isn't visible text.
    if (node.parentNode === root && !node.data.trim()) continue
    const fragment = document.createDocumentFragment()
    for (const grapheme of graphemes(node.data)) {
      const char = span('tome-tw', grapheme)
      char.style.setProperty('--tome-tw-index', String(index++))
      fragment.append(char)
    }
    node.replaceWith(fragment)
  }
  return index
}

/**
 * Prepare a block's HTML for effects that animate individual words or characters
 * (typewriter entrance; wave, tremble and inline breathe resting effects).
 *
 * The animated copy is hidden from assistive technology, and an intact copy is kept for
 * screen readers, since per-character spans are read out letter by letter.
 */
export function prepareKinetic(block: Block): void {
  const typewriter = block.reveal?.effect === 'typewriter'
  const template = document.createElement('template')
  template.innerHTML = block.html

  const toSplit = [...template.content.querySelectorAll<HTMLElement>('[data-tome-rest]')].filter((element) => {
    const effect = element.dataset.tomeRest ?? ''
    return PER_CHARACTER.has(effect) || (effect === 'breathe' && INLINE_TAGS.has(element.tagName))
  })
  if (!typewriter && toSplit.length === 0) return

  const counter = { index: 0 }
  for (const element of toSplit) splitWords(element, PER_CHARACTER.has(element.dataset.tomeRest!), counter)

  // Footnote markers in the animated copy still open on a click, but keyboard focus goes to
  // the intact copy's, which assistive technology can see.
  for (const marker of template.content.querySelectorAll('[data-tome-footnote]')) marker.setAttribute('tabindex', '-1')

  let style = ''
  if (typewriter) {
    const count = splitTypewriter(template.content)
    style = ` style="--tome-tw-count: ${Math.max(count, 1)}"`
    block.reveal!.duration_ms ??= count * TYPEWRITER_CHAR_MS
  }

  block.html =
    `<div class="tome-sr-only">${block.html}</div>` +
    `<div class="tome-visual" aria-hidden="true"${style}>${template.innerHTML}</div>`
}
