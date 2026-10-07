// @vitest-environment happy-dom
import { describe, expect, it } from 'vitest'
import { placeFootnote, visibleMarker } from './footnotes'

const viewport = { width: 800, height: 600 }
const popover = { width: 300, height: 100 }

describe('placeFootnote', () => {
  it('centres the popover below its marker', () => {
    expect(placeFootnote({ left: 400, top: 100, width: 10, height: 20 }, popover, viewport)).toEqual({ left: 255, top: 126 })
  })

  it('keeps the popover inside the window', () => {
    expect(placeFootnote({ left: 5, top: 100, width: 10, height: 20 }, popover, viewport).left).toBe(12)
    expect(placeFootnote({ left: 790, top: 100, width: 10, height: 20 }, popover, viewport).left).toBe(488)
  })

  it('goes above the marker when there is no room below', () => {
    expect(placeFootnote({ left: 400, top: 540, width: 10, height: 20 }, popover, viewport).top).toBe(434)
  })

  it('stays below when there is no room above either', () => {
    const tall = { width: 300, height: 500 }
    expect(placeFootnote({ left: 400, top: 300, width: 10, height: 20 }, tall, viewport).top).toBe(326)
  })
})

describe('visibleMarker', () => {
  it('anchors to the animated copy of a marker in the screen-reader copy', () => {
    document.body.innerHTML = `<div class="tome-block">
      <div class="tome-sr-only"><button id="hidden" data-tome-footnote="2">2</button></div>
      <div class="tome-visual"><button id="shown" data-tome-footnote="2">2</button></div>
    </div>`
    expect(visibleMarker(document.getElementById('hidden')!).id).toBe('shown')
    expect(visibleMarker(document.getElementById('shown')!).id).toBe('shown')
  })
})
