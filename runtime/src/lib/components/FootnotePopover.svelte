<script lang="ts">
  import type { Footnote } from '../bundle/types'
  import { placeFootnote } from '../reader/footnotes'

  interface Props {
    footnote: Footnote
    /** The marker it opened from, which it's anchored to. */
    anchor: HTMLElement
  }

  let { footnote, anchor }: Props = $props()
  let element: HTMLElement
  let position = $state<{ left: number; top: number } | null>(null)

  function place() {
    const viewport = { width: window.innerWidth, height: window.innerHeight }
    position = placeFootnote(anchor.getBoundingClientRect(), element.getBoundingClientRect(), viewport)
  }

  // Follow the marker as the page scrolls or the window resizes; move focus in so the
  // footnote is read out.
  $effect(() => {
    place()
    element.focus({ preventScroll: true })
    window.addEventListener('scroll', place, true)
    window.addEventListener('resize', place)
    return () => {
      window.removeEventListener('scroll', place, true)
      window.removeEventListener('resize', place)
    }
  })
</script>

<div
  class="tome-footnote"
  role="dialog"
  aria-label="Footnote {footnote.number}"
  tabindex="-1"
  bind:this={element}
  style:left={position ? `${position.left}px` : undefined}
  style:top={position ? `${position.top}px` : undefined}
  style:opacity={position ? undefined : '0'}
>
  <span class="tome-footnote-number" aria-hidden="true">{footnote.number}</span>
  <div class="tome-footnote-body">{@html footnote.html}</div>
</div>
