/** Space between a footnote marker and its popover, and the least space kept from the window's edge. */
const GAP = 6
const MARGIN = 12

interface Box {
  left: number
  top: number
  width: number
  height: number
}

/**
 * Where a footnote's popover goes: centred below its marker, above it when there's no room
 * below, and always inside the window.
 */
export function placeFootnote(marker: Box, popover: { width: number; height: number }, viewport: { width: number; height: number }) {
  const center = marker.left + marker.width / 2
  const left = Math.max(MARGIN, Math.min(center - popover.width / 2, viewport.width - MARGIN - popover.width))
  const below = marker.top + marker.height + GAP
  const above = marker.top - GAP - popover.height
  const fitsBelow = below + popover.height <= viewport.height - MARGIN
  const top = fitsBelow || above < MARGIN ? below : above
  return { left, top }
}

/**
 * The marker to anchor a footnote to. Per-character effects keep an intact copy of the text for
 * screen readers beside the animated one; its markers can take focus but aren't visible, so the
 * popover is anchored to the matching marker in the animated copy.
 */
export function visibleMarker(marker: HTMLElement): HTMLElement {
  if (!marker.closest('.tome-sr-only')) return marker
  const number = marker.dataset.tomeFootnote ?? ''
  const block = marker.closest('.tome-block')
  return block?.querySelector<HTMLElement>(`.tome-visual [data-tome-footnote="${CSS.escape(number)}"]`) ?? marker
}
