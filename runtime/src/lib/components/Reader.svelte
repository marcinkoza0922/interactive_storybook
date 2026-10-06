<script lang="ts">
  import { tick, untrack } from 'svelte'
  import { SvelteSet } from 'svelte/reactivity'
  import type { Bundle } from '../bundle/types'
  import { advance, back, pageAt, type Position, type ReaderState } from '../reader/navigation'
  import { pageReferences, referenceEntry } from '../references/gating'
  import PageView from './PageView.svelte'
  import ReferencePanel from './ReferencePanel.svelte'

  interface Props {
    bundle: Bundle
    initial: ReaderState
    /** Index of the furthest chapter reached; gates references. */
    furthestChapter: number
    assetUrl: (src: string) => string
    onpositionchange: (position: Position) => void
    /** Every navigation, including reveal steps; drives audio. */
    onchange: (reader: ReaderState, change: 'turn' | 'reveal') => void
    onexit: () => void
  }

  let { bundle, initial, furthestChapter, assetUrl, onpositionchange, onchange, onexit }: Props = $props()

  let reader = $state(untrack(() => initial))
  /** Blocks whose entrance animation is still running. */
  const entering = new SvelteSet<string>()
  let scroller: HTMLElement
  let panelToggle: HTMLButtonElement

  let panelOpen = $state(false)
  let selectedReference = $state<string | null>(null)

  const chapter = $derived(bundle.chapters[reader.position.chapter])
  const page = $derived(pageAt(bundle, reader.position))
  const references = $derived(pageReferences(bundle, page, furthestChapter))
  const selected = $derived(selectedReference ? referenceEntry(bundle, selectedReference, furthestChapter) : null)

  async function go(next: ReaderState | null) {
    if (!next) return
    const turned =
      next.position.chapter !== reader.position.chapter || next.position.page !== reader.position.page

    entering.clear()
    reader = next
    onchange(next, turned ? 'turn' : 'reveal')

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

  async function setPanelOpen(open: boolean) {
    panelOpen = open
    // An open entry survives page turns, but closing the panel returns it to the page's list.
    if (!open) selectedReference = null
    await tick()
    // Move focus into the panel when it opens, and back to its toggle when it closes.
    if (open) document.querySelector<HTMLElement>('.tome-sidebar button')?.focus()
    else panelToggle.focus()
  }

  function selectReference(id: string | null) {
    selectedReference = id
    tick().then(() => document.querySelector<HTMLElement>('.tome-sidebar button')?.focus())
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
      case 'r':
      case 'R':
        setPanelOpen(!panelOpen)
        break
      case 'Escape':
        if (panelOpen) setPanelOpen(false)
        else onexit()
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
    // Where the sidebar overlays the page, a tap outside it dismisses it rather than turning the page.
    if (panelOpen && matchMedia('(max-width: 48rem)').matches) {
      setPanelOpen(false)
      return
    }

    const bounds = (event.currentTarget as HTMLElement).getBoundingClientRect()
    if (event.clientX - bounds.left < bounds.width / 3) onBack()
    else onAdvance()
  }
</script>

<svelte:window {onkeydown} />

<div class="tome-reading">
  <div class="tome-reading-main">
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

    <footer class="tome-status">
      <span aria-live="polite">{chapter.title}</span>
      <button
        class="tome-link-button"
        bind:this={panelToggle}
        onclick={() => setPanelOpen(!panelOpen)}
        aria-expanded={panelOpen}
        aria-controls="tome-references"
      >
        References{references.length > 0 ? ` · ${references.length}` : ''}
      </button>
      <span>{reader.position.page + 1} / {chapter.pages.length}</span>
    </footer>
  </div>

  {#if panelOpen}
    <ReferencePanel
      id="tome-references"
      entries={references}
      {selected}
      onselect={selectReference}
      onclose={() => setPanelOpen(false)}
      {assetUrl}
    />
  {/if}
</div>
