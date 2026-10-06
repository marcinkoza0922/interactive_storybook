// @vitest-environment happy-dom
import { beforeEach, describe, expect, it } from 'vitest'
import { anchorSelection, anchoredText, highlightRanges } from './anchor'

let page: HTMLElement

beforeEach(() => {
  document.body.innerHTML = `
    <article class="tome-page">
      <div class="tome-block" data-block-id="a"><p>The ferry came <em>in late</em>.</p></div>
      <div class="tome-block" data-block-id="b">
        <div class="tome-sr-only"><p>Oh, wonderful.</p></div><div class="tome-visual" aria-hidden="true"><p>Oh, <span><span class="tome-char">w</span><span class="tome-char">o</span>nderful</span>.</p></div>
      </div>
      <div class="tome-block tome-hidden" data-block-id="c"><p>Not yet.</p></div>
    </article>`
  page = document.querySelector('.tome-page')!
})

function select(startNode: Node, startOffset: number, endNode: Node, endOffset: number): Selection {
  const range = document.createRange()
  range.setStart(startNode, startOffset)
  range.setEnd(endNode, endOffset)
  const selection = window.getSelection()!
  selection.removeAllRanges()
  selection.addRange(range)
  return selection
}

const text = (selector: string) => document.querySelector(selector)!.firstChild!

describe('highlight anchors', () => {
  it('anchors a selection across markup within one block', () => {
    // "ferry came in" — from "ferry" in the paragraph text into the <em>
    const anchor = anchorSelection(page, select(text('[data-block-id="a"] p'), 4, text('[data-block-id="a"] em'), 2))
    expect(anchor).toEqual({ start: { block_id: 'a', offset: 4 }, end: { block_id: 'a', offset: 17 }, text: 'ferry came in' })
  })

  it('uses the visible copy of split text, ignoring the screen-reader copy', () => {
    const visualP = document.querySelector('[data-block-id="b"] .tome-visual p')!
    const anchor = anchorSelection(page, select(visualP.firstChild!, 0, document.querySelectorAll('.tome-char')[1].firstChild!, 1))
    expect(anchor?.text).toBe('Oh, wo')
    expect(anchor?.end).toEqual({ block_id: 'b', offset: 6 })
  })

  it('spans blocks, one line per block, and paints one range per block', () => {
    const visualP = document.querySelector('[data-block-id="b"] .tome-visual p')!
    const anchor = anchorSelection(page, select(text('[data-block-id="a"] em'), 3, visualP.firstChild!, 2))!
    expect(anchor.text).toBe('late.\nOh')

    const ranges = highlightRanges(page, anchor)
    expect(ranges.map((r) => r.toString())).toEqual(['late.', 'Oh'])
  })

  it('ignores selections in unrevealed blocks', () => {
    expect(anchorSelection(page, select(text('[data-block-id="c"] p'), 0, text('[data-block-id="c"] p'), 3))).toBeNull()
  })

  it('paints nothing when the text no longer matches', () => {
    const stale = { start: { block_id: 'a', offset: 4 }, end: { block_id: 'a', offset: 9 }, text: 'boat!' }
    expect(anchoredText(page, stale.start, stale.end)).toBe('ferry')
    expect(highlightRanges(page, stale)).toEqual([])
  })
})
