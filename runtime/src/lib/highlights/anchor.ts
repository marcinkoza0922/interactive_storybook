import type { Highlight, TextAnchor } from '../storage/save'

export type HighlightAnchor = Pick<Highlight, 'start' | 'end' | 'text'>

/** The element holding the text the reader sees: the animated copy, if the block has one. */
export function visibleTextRoot(block: Element): Element {
  return block.querySelector('.tome-visual') ?? block
}

function blocksOf(page: Element): Element[] {
  return [...page.querySelectorAll('.tome-block[data-block-id]')]
}

function blockOf(node: Node): Element | null {
  return (node instanceof Element ? node : node.parentElement)?.closest('.tome-block[data-block-id]') ?? null
}

/** Character offset of a DOM point within `root`'s text, clamped to its bounds. */
function textOffset(root: Element, node: Node, offset: number, atEnd: boolean): number {
  if (!root.contains(node)) return atEnd ? (root.textContent ?? '').length : 0
  const range = document.createRange()
  range.setStart(root, 0)
  range.setEnd(node, offset)
  return range.toString().length
}

/** The text between two anchors on a page, one line per block. Null if they don't apply. */
export function anchoredText(page: Element, start: TextAnchor, end: TextAnchor): string | null {
  const blocks = blocksOf(page)
  const first = blocks.findIndex((b) => b.getAttribute('data-block-id') === start.block_id)
  const last = blocks.findIndex((b) => b.getAttribute('data-block-id') === end.block_id)
  if (first === -1 || last === -1 || last < first) return null

  return blocks
    .slice(first, last + 1)
    .map((block, i) => {
      const text = visibleTextRoot(block).textContent ?? ''
      return text.slice(i === 0 ? start.offset : 0, first + i === last ? end.offset : text.length)
    })
    .join('\n')
}

/** Turn the current selection into anchors, if it lies within visible blocks of `page`. */
export function anchorSelection(page: Element, selection: Selection | null): HighlightAnchor | null {
  if (!selection || selection.isCollapsed || selection.rangeCount === 0) return null
  const range = selection.getRangeAt(0)
  const startBlock = blockOf(range.startContainer)
  const endBlock = blockOf(range.endContainer)
  if (!startBlock || !endBlock || !page.contains(startBlock) || !page.contains(endBlock)) return null
  if (startBlock.classList.contains('tome-hidden') || endBlock.classList.contains('tome-hidden')) return null

  const start = {
    block_id: startBlock.getAttribute('data-block-id')!,
    offset: textOffset(visibleTextRoot(startBlock), range.startContainer, range.startOffset, false),
  }
  const end = {
    block_id: endBlock.getAttribute('data-block-id')!,
    offset: textOffset(visibleTextRoot(endBlock), range.endContainer, range.endOffset, true),
  }
  const text = anchoredText(page, start, end)
  return text?.trim() ? { start, end, text } : null
}

/** A DOM range covering characters [from, to) of `root`'s text. */
export function rangeIn(root: Element, from: number, to: number): Range | null {
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT)
  const range = document.createRange()
  let seen = 0
  let started = false
  while (walker.nextNode()) {
    const node = walker.currentNode as Text
    const length = node.data.length
    if (!started && from <= seen + length) {
      range.setStart(node, from - seen)
      started = true
    }
    if (started && to <= seen + length) {
      range.setEnd(node, to - seen)
      return range
    }
    seen += length
  }
  return null
}

/**
 * The ranges to paint for a highlight, one per block so that copies kept for screen readers
 * are skipped. Empty if the page's text no longer matches what was highlighted.
 */
export function highlightRanges(page: Element, highlight: HighlightAnchor): Range[] {
  if (anchoredText(page, highlight.start, highlight.end) !== highlight.text) return []

  const blocks = blocksOf(page)
  const first = blocks.findIndex((b) => b.getAttribute('data-block-id') === highlight.start.block_id)
  const last = blocks.findIndex((b) => b.getAttribute('data-block-id') === highlight.end.block_id)

  return blocks.slice(first, last + 1).flatMap((block, i) => {
    const root = visibleTextRoot(block)
    const length = (root.textContent ?? '').length
    const from = i === 0 ? highlight.start.offset : 0
    const to = first + i === last ? highlight.end.offset : length
    const range = from < to ? rangeIn(root, from, to) : null
    return range ? [range] : []
  })
}
