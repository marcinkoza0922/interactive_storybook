<script lang="ts">
  import { onDestroy, tick, untrack } from 'svelte'
  import { MediaQuery, SvelteSet } from 'svelte/reactivity'
  import type { Bundle, Footnote } from '../bundle/types'
  import { DEFAULT_LINGER_MS, introducesIllustration, trackStates } from '../illustrations/track'
  import type { Captions } from '../audio/captions.svelte'
  import type { NarrationProgress } from '../audio/director'
  import { anchorSelection, highlightRanges, rangeIn, visibleTextRoot, type HighlightAnchor } from '../highlights/anchor'
  import { visibleMarker } from '../reader/footnotes'
  import {
    advance,
    arriveBackward,
    back,
    pageAt,
    stepCount,
    type Position,
    type ReaderState,
  } from '../reader/navigation'
  import { pageReferences, referenceEntry } from '../references/gating'
  import type { SearchMatch } from '../search/search'
  import type { Progress } from '../state/progress.svelte'
  import { arriveByJump, isHighlightCurrent } from '../storage/save'
  import CaptionStrip from './CaptionStrip.svelte'
  import FootnotePopover from './FootnotePopover.svelte'
  import Menu from './Menu.svelte'
  import PageView from './PageView.svelte'
  import ReferencePanel from './ReferencePanel.svelte'

  interface Props {
    bundle: Bundle
    initial: ReaderState
    progress: Progress
    /** Narration is in progress, heard or passing silently: auto mode waits for it. */
    narrating: boolean
    /** The reader can hear narration: only then does turning the page ask to confirm. */
    narrationAudible: boolean
    /** Captions of the sounds playing, shown when the reader turns them on. */
    captions: Captions
    /** Where the narration being heard is, for highlighting what it reads. */
    narrationProgress: () => NarrationProgress | null
    assetUrl: (src: string) => string
    /** Every navigation, including reveal steps; drives audio. */
    onchange: (reader: ReaderState, change: 'turn' | 'reveal') => void
    onexit: () => void
  }

  let {
    bundle,
    initial,
    progress,
    narrating,
    narrationAudible,
    captions,
    narrationProgress,
    assetUrl,
    onchange,
    onexit,
  }: Props = $props()
  const narrationHeard = $derived(narrating && narrationAudible)

  let reader = $state(untrack(() => initial))
  /** Blocks whose entrance animation is still running. */
  const entering = new SvelteSet<string>()
  let scroller: HTMLElement
  let panelToggle: HTMLButtonElement

  let panelOpen = $state(false)
  let selectedReference = $state<string | null>(null)
  let menuOpen = $state(false)
  /** Open the menu straight to search, which Back then closes. */
  let menuStart = $state<'search' | undefined>()
  /** The last search, still there when Search opens again. */
  let searchQuery = $state('')
  /** Shown after a first attempt to turn the page during narration. */
  let turnNotice = $state(false)
  let turnNoticeTimer: ReturnType<typeof setTimeout> | undefined
  const TURN_CONFIRM_MS = 4000
  /** A brief message confirming an action, e.g. adding a bookmark. */
  let hint = $state<string | null>(null)
  let hintTimer: ReturnType<typeof setTimeout> | undefined
  const HINT_MS = 2500
  /** The open footnote, and the marker it opened from. */
  let footnote = $state<{ footnote: Footnote; marker: HTMLElement } | null>(null)
  /** A text selection that can be turned into a highlight, and where to offer it. */
  let pendingHighlight = $state<{ anchor: HighlightAnchor; x: number; y: number } | null>(null)

  /** Wide, landscape viewports show the illustration track as a facing page. */
  const wide = new MediaQuery('(min-width: 60rem) and (min-aspect-ratio: 5/4)')
  const track = trackStates(untrack(() => bundle))
  /** On narrow viewports the reader sees either the text or the track illustration. */
  let narrowView = $state<'text' | 'illustration'>('text')
  /** The text is fading in after the illustration lingered. */
  let textFadingIn = $state(false)
  let lingerTimer: ReturnType<typeof setTimeout> | undefined

  const chapter = $derived(bundle.chapters[reader.position.chapter])
  const page = $derived(pageAt(bundle, reader.position))
  const illustration = $derived(track[reader.position.chapter][reader.position.page])
  const layout = $derived(illustration && wide.current ? 'spread' : 'single')
  const showingIllustration = $derived(illustration !== null && !wide.current && narrowView === 'illustration')
  const unlocked = $derived(progress.unlockedChapter)
  const references = $derived(pageReferences(bundle, page, unlocked))
  const selected = $derived(selectedReference ? referenceEntry(bundle, selectedReference, unlocked) : null)
  const bookmarked = $derived(progress.bookmarkAt(reader.position) !== undefined)
  const pageHighlights = $derived(
    progress.save.highlights.filter(
      (h) => h.chapter_id === chapter.id && h.page === reader.position.page && isHighlightCurrent(bundle, h),
    ),
  )

  /**
   * Arriving forward on a page that introduces a new track illustration lingers on it
   * (on narrow viewports) before fading in the text. Going back shows the text at once.
   */
  function arrive(state: ReaderState) {
    clearTimeout(lingerTimer)
    textFadingIn = false
    if (!wide.current && state.direction === 'forward' && introducesIllustration(track, state.position)) {
      narrowView = 'illustration'
      lingerTimer = setTimeout(showText, bundle.illustrations?.linger_ms ?? DEFAULT_LINGER_MS)
    } else {
      narrowView = 'text'
    }
  }

  function showText() {
    clearTimeout(lingerTimer)
    if (narrowView === 'text') return
    narrowView = 'text'
    textFadingIn = true
  }

  function toggleIllustration() {
    if (narrowView === 'text') {
      clearTimeout(lingerTimer)
      narrowView = 'illustration'
    } else {
      showText()
    }
  }

  arrive(untrack(() => initial))
  onDestroy(() => clearTimeout(lingerTimer))

  async function go(next: ReaderState | null) {
    if (!next) return
    const turned =
      next.position.chapter !== reader.position.chapter || next.position.page !== reader.position.page

    entering.clear()
    reader = next
    onchange(next, turned ? 'turn' : 'reveal')

    if (turned) {
      closeFootnote(false)
      clearSearchMark()
      arrive(next)
      pendingHighlight = null
      progress.setPosition(next.position)
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

  /**
   * While narration plays, the first attempt to turn the page only warns; a second within a
   * few seconds turns it (cutting the narration off). Revealing a step never asks.
   */
  function confirmTurn(): boolean {
    if (!narrationHeard || turnNotice) {
      dismissTurnNotice()
      return true
    }
    turnNotice = true
    turnNoticeTimer = setTimeout(dismissTurnNotice, TURN_CONFIRM_MS)
    return false
  }

  function dismissTurnNotice() {
    clearTimeout(turnNoticeTimer)
    turnNotice = false
  }

  // Once the narration ends (or is muted), turning needs no confirmation.
  $effect(() => {
    if (!narrationHeard) dismissTurnNotice()
  })
  onDestroy(() => clearTimeout(turnNoticeTimer))

  function showHint(message: string) {
    clearTimeout(hintTimer)
    hint = message
    hintTimer = setTimeout(() => (hint = null), HINT_MS)
  }
  onDestroy(() => clearTimeout(hintTimer))

  function onAdvance() {
    // Advancing from the illustration (lingering or toggled) returns to the text.
    if (showingIllustration) {
      showText()
      return
    }
    // Advancing while an animation runs completes it instead of moving on.
    if (entering.size > 0) {
      entering.clear()
      return
    }
    const next = advance(bundle, reader)
    const turns = next !== null && (next.position.chapter !== reader.position.chapter || next.position.page !== reader.position.page)
    if (turns && !confirmTurn()) return
    go(next)
  }

  function onBack() {
    const previous = back(bundle, reader)
    if (previous && !confirmTurn()) return
    go(previous)
  }

  /** Activating a marker opens its footnote, or closes it if it's open; it never turns the page. */
  function toggleFootnote(marker: HTMLElement) {
    const number = Number(marker.dataset.tomeFootnote)
    const open = footnote?.footnote.number === number
    closeFootnote(false)
    const found = page.footnotes?.find((f) => f.number === number)
    if (open || !found) return
    marker.setAttribute('aria-expanded', 'true')
    footnote = { footnote: found, marker }
  }

  function closeFootnote(returnFocus: boolean) {
    if (!footnote) return
    footnote.marker.removeAttribute('aria-expanded')
    if (returnFocus) footnote.marker.focus()
    footnote = null
  }

  /** A click outside an open footnote closes it. On the page, that's all the click does. */
  function onwindowclick(event: MouseEvent) {
    const target = event.target as Element
    if (!footnote || target.closest('.tome-footnote, [data-tome-footnote]')) return
    closeFootnote(false)
    if (!target.closest('a, button, input, textarea, select')) event.stopPropagation()
  }

  /** Go to a page from the menu. Pages already read are shown fully revealed. */
  function jump(position: Position) {
    menuOpen = false
    if (position.chapter === reader.position.chapter && position.page === reader.position.page) return
    go(arriveByJump(bundle, progress.save, position))
  }

  function openMenu(start?: 'search') {
    menuStart = start
    menuOpen = true
  }

  /** A match found by search is marked briefly after going to it. */
  let searchMarkTimer: ReturnType<typeof setTimeout> | undefined
  const SEARCH_MARK_MS = 2500

  function clearSearchMark() {
    clearTimeout(searchMarkTimer)
    if ('highlights' in CSS) CSS.highlights.delete('tome-search')
  }
  onDestroy(clearSearchMark)

  /**
   * Go to a search match. It's in a chapter already reached (or a book already read), so the
   * page shows fully revealed. A match in a footnote opens it.
   */
  async function find(match: SearchMatch) {
    menuOpen = false
    const position = { chapter: match.chapter, page: match.page }
    const samePage = position.chapter === reader.position.chapter && position.page === reader.position.page
    if (!samePage || reader.revealed < stepCount(page)) go(arriveBackward(bundle, position))
    await tick()

    let root: Element | null = null
    if (match.target.kind === 'footnote') {
      const number = String(match.target.number)
      const marker = scroller.querySelector<HTMLElement>(`[data-tome-footnote="${CSS.escape(number)}"]`)
      if (!marker) return
      visibleMarker(marker).scrollIntoView({ block: 'center' })
      if (footnote?.footnote.number !== match.target.number) toggleFootnote(marker)
      await tick()
      root = document.querySelector('.tome-footnote-body')
    } else {
      const block = scroller.querySelector(`[data-block-id="${CSS.escape(match.target.blockId)}"]`)
      block?.scrollIntoView({ block: 'center' })
      root = block && visibleTextRoot(block)
    }

    const range = root && rangeIn(root, match.start, match.end)
    if (!range || !('highlights' in CSS)) return
    clearSearchMark()
    CSS.highlights.set('tome-search', new Highlight(range))
    searchMarkTimer = setTimeout(clearSearchMark, SEARCH_MARK_MS)
  }

  function toggleBookmark() {
    progress.toggleBookmark(reader.position)
    showHint(bookmarked ? 'Page bookmarked' : 'Bookmark removed')
  }

  /** From the status bar, with nothing selected, explain how to highlight instead. */
  function highlightOrExplain() {
    if (pendingHighlight) createHighlight()
    else showHint('Select text on the page to highlight it')
  }

  function createHighlight() {
    if (!pendingHighlight) return
    progress.addHighlight(reader.position, pendingHighlight.anchor)
    document.getSelection()?.removeAllRanges()
    pendingHighlight = null
  }

  function onselectionchange() {
    const article = scroller?.querySelector('.tome-page')
    const selection = document.getSelection()
    const anchor = article && !menuOpen ? anchorSelection(article, selection) : null
    if (!anchor) {
      pendingHighlight = null
      return
    }
    const rect = selection!.getRangeAt(0).getBoundingClientRect()
    const x = Math.min(Math.max(rect.left + rect.width / 2, 64), window.innerWidth - 64)
    pendingHighlight = { anchor, x, y: Math.min(rect.bottom + 8, window.innerHeight - 48) }
  }

  // Paint saved highlights on the current page. CSS Custom Highlights mark text without
  // touching the DOM, which per-character effects have already restructured.
  $effect(() => {
    const highlights = pageHighlights
    void page
    if (!('highlights' in CSS)) return
    const article = scroller.querySelector('.tome-page')
    const ranges = article ? highlights.flatMap((h) => highlightRanges(article, h)) : []
    CSS.highlights.set('tome-highlight', new Highlight(...ranges))
  })
  onDestroy(() => ('highlights' in CSS ? CSS.highlights.delete('tome-highlight') : undefined))

  // Narration highlighting: while narration is heard, mark the paragraph it reads and, with
  // word timings, the word being spoken. Words are painted with a CSS Custom Highlight, which
  // leaves the DOM alone (per-character effects have already split the text).
  let narratedBlock: Element | null = null
  let litWord = -1
  let wordRanges = new Map<string, Range[]>()

  function clearNarrationMarks() {
    narratedBlock?.removeAttribute('data-narrating')
    narratedBlock = null
    litWord = -1
    if ('highlights' in CSS) CSS.highlights.delete('tome-narration')
  }

  function words(block: Element, id: string): Range[] {
    let ranges = wordRanges.get(id)
    if (!ranges) {
      const root = visibleTextRoot(block)
      const text = root.textContent ?? ''
      ranges = [...text.matchAll(/\S+/g)].flatMap((m) => rangeIn(root, m.index, m.index + m[0].length) ?? [])
      wordRanges.set(id, ranges)
    }
    return ranges
  }

  function paintNarration() {
    const now = narrationProgress()
    const block = now && scroller.querySelector(`[data-block-id="${CSS.escape(now.blockId)}"]`)
    if (!now || !block) return clearNarrationMarks()
    if (block !== narratedBlock) {
      clearNarrationMarks()
      narratedBlock = block
      block.setAttribute('data-narrating', now.words ? 'words' : 'paragraph')
    }
    if (!now.words || !('highlights' in CSS)) return

    let index = -1
    while (index + 1 < now.words.length && (now.words[index + 1] ?? Infinity) <= now.ms) index++
    if (index === litWord) return
    litWord = index
    const range = words(block, now.blockId)[index]
    if (range) CSS.highlights.set('tome-narration', new Highlight(range))
    else CSS.highlights.delete('tome-narration')
  }

  $effect(() => {
    void page // a new page means new text to find
    wordRanges = new Map()
    if (!narrating || !narrationAudible || !progress.settings.narration_highlight) return clearNarrationMarks()
    let frame = requestAnimationFrame(function tick() {
      paintNarration()
      frame = requestAnimationFrame(tick)
    })
    return () => {
      cancelAnimationFrame(frame)
      clearNarrationMarks()
    }
  })

  // Auto mode: advance on a fixed timer, restarted by anything that changes what's on screen.
  // Paused while the menu or references are open, and while narration is in progress, even
  // silently: a line's length is a good guide to how long its text takes to read.
  $effect(() => {
    if (!progress.settings.auto_advance || menuOpen || panelOpen || footnote || narrating) return
    void reader
    void entering.size
    void narrowView
    const timer = setTimeout(onAdvance, progress.settings.auto_interval_s * 1000)
    return () => clearTimeout(timer)
  })

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
    // The menu is a modal dialog: it handles its own keys, including Esc to close.
    if (menuOpen) return
    const findKey = event.key === '/' || ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'f')
    if (findKey && !event.altKey && !(event.target as Element | null)?.closest('input, textarea, select')) {
      event.preventDefault()
      openMenu('search')
      return
    }
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
      case 'i':
      case 'I':
        if (illustration && !wide.current) toggleIllustration()
        break
      case 'b':
      case 'B':
        toggleBookmark()
        break
      case 'h':
      case 'H':
        createHighlight()
        break
      case 'Escape':
        if (footnote) closeFootnote(true)
        else if (panelOpen) setPanelOpen(false)
        else openMenu()
        break
      default:
        return
    }
    event.preventDefault()
  }

  /** Click or tap: the left third goes back, the rest advances. */
  function onclick(event: MouseEvent) {
    const marker = (event.target as Element).closest<HTMLElement>('[data-tome-footnote]')
    if (marker) return toggleFootnote(marker)
    if ((event.target as Element).closest('a, button, .tome-status')) return
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

<svelte:window {onkeydown} onclickcapture={onwindowclick} />
<svelte:document {onselectionchange} />

<div class="tome-reading" data-layout={layout} data-chapter={chapter.id} data-paper={page.paper}>
  <!-- Keyboard input is handled at the window level; clicking is a pointer convenience. -->
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="tome-stage" {onclick}>
    {#if illustration && (layout === 'spread' || showingIllustration)}
      <figure class="tome-illustration">
        {#key illustration.src}
          <img class="tome-illustration-image" src={assetUrl(illustration.src)} alt={illustration.alt} />
        {/key}
      </figure>
    {/if}

    <div class="tome-text-column">
      <main class="tome-reader" bind:this={scroller} hidden={showingIllustration}>
        <PageView
          {page}
          chapterTitle={reader.position.page === 0 ? chapter.title : null}
          chapterImage={reader.position.page === 0 ? chapter.header_image : undefined}
          fadeIn={textFadingIn}
          revealed={reader.revealed}
          {entering}
          onentered={(id) => entering.delete(id)}
          {assetUrl}
        />
      </main>

      {#if progress.settings.captions}
        <CaptionStrip {captions} />
      {/if}

      <footer class="tome-status">
        <span aria-live="polite">
          {chapter.title}
        </span>
        <span class="tome-status-actions">
          <button
            class="tome-link-button tome-status-icon"
            onclick={toggleBookmark}
            aria-pressed={bookmarked}
            aria-label="Bookmark this page"
            title={bookmarked ? 'Remove bookmark (B)' : 'Bookmark this page (B)'}
          >
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M6 3h12v18l-6-4.5L6 21z" fill={bookmarked ? 'currentColor' : 'none'} />
            </svg>
          </button>
          <button
            class="tome-link-button tome-status-icon"
            onpointerdown={(e) => e.preventDefault()}
            onclick={highlightOrExplain}
            aria-label="Highlight selected text"
            title="Highlight selected text (H)"
          >
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M15 4l5 5-9 9H6v-5z M4 21h16" fill="none" />
            </svg>
          </button>
          {#if illustration && layout === 'single'}
            <button class="tome-link-button" onclick={toggleIllustration}>
              {showingIllustration ? 'Text' : 'Illustration'}
            </button>
          {/if}
          <button
            class="tome-link-button"
            bind:this={panelToggle}
            onclick={() => setPanelOpen(!panelOpen)}
            aria-expanded={panelOpen}
            aria-controls="tome-references"
          >
            References{references.length > 0 ? ` · ${references.length}` : ''}
          </button>
          <button class="tome-link-button" onclick={() => openMenu()} aria-haspopup="dialog">Menu</button>
        </span>
        <span>{reader.position.page + 1} / {chapter.pages.length}</span>
      </footer>
    </div>
  </div>

  {#if footnote}
    {#key footnote.footnote.number}
      <FootnotePopover footnote={footnote.footnote} anchor={visibleMarker(footnote.marker)} />
    {/key}
  {/if}

  {#if turnNotice}
    <p class="tome-toast" role="status">Narration is still playing — press again to turn the page.</p>
  {:else if hint}
    <p class="tome-toast" role="status">{hint}</p>
  {/if}

  {#if pendingHighlight}
    <button
      class="tome-button tome-button-primary tome-highlight-button"
      style:left="{pendingHighlight.x}px"
      style:top="{pendingHighlight.y}px"
      onpointerdown={(e) => e.preventDefault()}
      onclick={createHighlight}
    >
      Highlight
    </button>
  {/if}

  {#if menuOpen}
    <Menu
      {bundle}
      {progress}
      position={reader.position}
      start={menuStart}
      onclose={() => (menuOpen = false)}
      onjump={jump}
      onfind={find}
      bind:searchQuery
      onreferences={() => {
        menuOpen = false
        setPanelOpen(true)
      }}
      ontitle={onexit}
      onreset={onexit}
    />
  {/if}

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
