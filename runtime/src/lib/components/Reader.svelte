<script lang="ts">
  import { tick, untrack } from 'svelte'
  import { SvelteSet } from 'svelte/reactivity'
  import type { Bundle } from '../bundle/types'
  import { advance, back, pageAt, type Position, type ReaderState } from '../reader/navigation'
  import PageView from './PageView.svelte'

  interface Props {
    bundle: Bundle
    initial: ReaderState
    onpositionchange: (position: Position) => void
    onexit: () => void
  }

  let { bundle, initial, onpositionchange, onexit }: Props = $props()

  let reader = $state(untrack(() => initial))
  /** Blocks whose entrance animation is still running. */
  const entering = new SvelteSet<string>()
  let scroller: HTMLElement

  const chapter = $derived(bundle.chapters[reader.position.chapter])
  const page = $derived(pageAt(bundle, reader.position))

  async function go(next: ReaderState | null) {
    if (!next) return
    const turned =
      next.position.chapter !== reader.position.chapter || next.position.page !== reader.position.page

    entering.clear()
    reader = next

    if (turned) {
      onpositionchange(next.position)
      scroller.scrollTop = 0
      return
    }

    const stepBlocks = page.blocks.filter((b) => b.reveal?.step === next.revealed)
    for (const block of stepBlocks) entering.add(block.id)

    // Keep newly revealed text on screen when a long page scrolls.
    await tick()
    scroller
      .querySelector(`[data-block-id="${CSS.escape(stepBlocks[0]?.id ?? '')}"]`)
      ?.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
  }

  function onAdvance() {
    // Advancing while an animation runs completes it instead of moving on.
    if (entering.size > 0) {
      entering.clear()
      return
    }
    go(advance(bundle, reader))
  }

  function onBack() {
    go(back(bundle, reader))
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.altKey || event.ctrlKey || event.metaKey || event.defaultPrevented) return
    // Let focused controls handle their own activation keys.
    const onControl = (event.target as Element | null)?.closest('button, a, input, textarea, select')
    if (onControl && (event.key === ' ' || event.key === 'Enter')) return

    switch (event.key) {
      case 'ArrowRight':
      case 'PageDown':
      case ' ':
      case 'Enter':
        onAdvance()
        break
      case 'ArrowLeft':
      case 'PageUp':
      case 'Backspace':
        onBack()
        break
      case 'Escape':
        onexit()
        break
      default:
        return
    }
    event.preventDefault()
  }

  /** Click or tap: the left third goes back, the rest advances. */
  function onclick(event: MouseEvent) {
    if ((event.target as Element).closest('a, button')) return
    // Don't turn the page when the reader is selecting text.
    if (!window.getSelection()?.isCollapsed) return

    const bounds = (event.currentTarget as HTMLElement).getBoundingClientRect()
    if (event.clientX - bounds.left < bounds.width / 3) onBack()
    else onAdvance()
  }
</script>

<svelte:window {onkeydown} />

<!-- Keyboard input is handled at the window level; clicking is a pointer convenience. -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<main class="tome-reader" bind:this={scroller} {onclick}>
  <PageView
    {page}
    chapterTitle={reader.position.page === 0 ? chapter.title : null}
    revealed={reader.revealed}
    {entering}
    onentered={(id) => entering.delete(id)}
  />
</main>

<footer class="tome-status" aria-live="polite">
  <span>{chapter.title}</span>
  <span>{reader.position.page + 1} / {chapter.pages.length}</span>
</footer>
