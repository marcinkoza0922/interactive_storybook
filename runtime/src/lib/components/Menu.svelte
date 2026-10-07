<script lang="ts">
  import { tick, untrack } from 'svelte'
  import type { Bundle } from '../bundle/types'
  import { desktop } from '../desktop.svelte'
  import type { Position } from '../reader/navigation'
  import { AUDIO_CHANNELS, BODY_FONTS } from '../settings/apply'
  import type { Progress } from '../state/progress.svelte'
  import {
    FONT_SCALE_RANGE,
    chapterLocked,
    isHighlightCurrent,
    resolvePosition,
    type BodyFont,
    type PageRef,
  } from '../storage/save'

  type List = 'chapters' | 'bookmarks' | 'highlights'

  interface Props {
    bundle: Bundle
    progress: Progress
    /** The page being read; null on the title screen. */
    position: Position | null
    /** Open straight to one list, which Back then closes; the main view never shows. */
    start?: List
    onclose: () => void
    /** Go to a page; `seen` pages are shown fully revealed. */
    onjump: (position: Position) => void
    /** These three are reached only through the main view. */
    onreferences?: () => void
    ontitle?: () => void
    /** Progress was reset: leave the book for the title screen. */
    onreset?: () => void
  }

  let { bundle, progress, position, start, onclose, onjump, onreferences, ontitle, onreset }: Props = $props()

  type View = 'main' | List | 'settings' | 'reset' | { confirmJump: Position; from: List }
  let deleteAnnotations = $state(false)
  let view = $state<View>(untrack(() => start) ?? 'main')
  let dialog: HTMLDialogElement

  const settings = $derived(progress.settings)
  const currentBookmark = $derived(position && progress.bookmarkAt(position))
  const highlights = $derived(progress.save.highlights.filter((h) => isHighlightCurrent(bundle, h)))
  const contents = $derived(
    bundle.contents ?? bundle.chapters.map((c) => ({ kind: 'chapter' as const, id: c.id, title: undefined })),
  )

  $effect(() => {
    dialog.showModal()
  })

  async function show(next: View) {
    view = next
    await tick()
    dialog.querySelector<HTMLElement>('.tome-menu-view button, .tome-menu-view input')?.focus()
  }

  function chapterTitle(id: string, override?: string): string {
    return override ?? bundle.chapters.find((c) => c.id === id)?.title ?? id
  }

  function pageLabel(ref: PageRef): string {
    return `${chapterTitle(ref.chapter_id)} · page ${ref.page + 1}`
  }

  /** Go to a page, first warning if it's in a chapter the reader hasn't reached. */
  function jumpTo(target: Position, from: List) {
    if (chapterLocked(bundle, progress.save, target.chapter)) show({ confirmJump: target, from })
    else onjump(target)
  }

  function reset() {
    progress.resetProgress({ annotations: deleteAnnotations })
    onreset?.()
  }

  /** Back leads to the parent view, or closes a menu opened straight to this one. */
  function back() {
    if (view === start) dialog.close()
    else if (typeof view === 'object') show(view.from)
    else show(view === 'reset' ? 'settings' : 'main')
  }

  function setVolume(channel: (typeof AUDIO_CHANNELS)[number]['channel'], volume: number) {
    progress.updateSettings({ audio: { ...settings.audio, [channel]: { ...settings.audio[channel], volume } } })
  }

  function setMuted(channel: (typeof AUDIO_CHANNELS)[number]['channel'], muted: boolean) {
    progress.updateSettings({ audio: { ...settings.audio, [channel]: { ...settings.audio[channel], muted } } })
  }
</script>

<dialog class="tome-menu" bind:this={dialog} onclose={onclose} aria-label="Menu">
  <button class="tome-link-button tome-menu-close" onclick={() => dialog.close()} aria-label="Close menu" title="Close (Esc)">
    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 6l12 12M18 6L6 18" /></svg>
  </button>
  <div class="tome-menu-view">
    {#if view === 'main'}
      <h2 class="tome-menu-heading">{bundle.book.title}</h2>
      <nav class="tome-menu-list tome-menu-main">
        <button class="tome-menu-item" onclick={() => show('chapters')}>Chapters</button>
        <button class="tome-menu-item" onclick={() => show('bookmarks')}>
          <span class="tome-menu-label">
            <svg class="tome-menu-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 3h12v18l-6-4.5L6 21z" /></svg>
            Bookmarks
          </span>
          <span class="tome-menu-count">{progress.save.bookmarks.length || ''}</span>
        </button>
        <button class="tome-menu-item" onclick={() => show('highlights')}>
          Highlights <span class="tome-menu-count">{highlights.length || ''}</span>
        </button>
        <button class="tome-menu-item" onclick={onreferences}>References</button>
        <button class="tome-menu-item" onclick={() => show('settings')}>
          <span class="tome-menu-label">
            <svg class="tome-menu-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path
                d="M19.1 9.3L21.6 10.1L21.6 13.9L19.1 14.7L18.9 15.1L20.2 17.4L17.4 20.2L15.1 18.9L14.7 19.1L13.9 21.6L10.1 21.6L9.3 19.1L8.9 18.9L6.6 20.2L3.8 17.4L5.1 15.1L4.9 14.7L2.4 13.9L2.4 10.1L4.9 9.3L5.1 8.9L3.8 6.6L6.6 3.8L8.9 5.1L9.3 4.9L10.1 2.4L13.9 2.4L14.7 4.9L15.1 5.1L17.4 3.8L20.2 6.6L18.9 8.9z M15 12a3 3 0 1 1-6 0 3 3 0 1 1 6 0z"
              />
            </svg>
            Settings
          </span>
        </button>
      </nav>
      <!-- Leaving the book sits apart from the reading options. -->
      <nav class="tome-menu-list tome-menu-leave">
        <button class="tome-menu-item" onclick={ontitle}>Title screen</button>
        {#if desktop}
          <button class="tome-menu-item" onclick={() => desktop?.quit()}>Exit</button>
        {/if}
      </nav>
    {:else}
      <button class="tome-link-button tome-menu-back" onclick={back}>
        ← Back
      </button>

      {#if view === 'chapters'}
        <h2 class="tome-menu-heading">Chapters</h2>
        <ol class="tome-menu-list tome-contents">
          {#each contents as entry, i (i)}
            {#if entry.kind === 'heading'}
              <li class="tome-contents-heading">{entry.title}</li>
            {:else}
              {@const chapter = bundle.chapters.findIndex((c) => c.id === entry.id)}
              {#if chapter !== -1}
                {@const locked = chapterLocked(bundle, progress.save, chapter)}
                <li>
                  <button
                    class="tome-menu-item"
                    class:tome-menu-locked={locked}
                    aria-current={chapter === position?.chapter ? 'true' : undefined}
                    onclick={() => jumpTo({ chapter, page: 0 }, 'chapters')}
                  >
                    {chapterTitle(entry.id, entry.title)}
                    {#if locked}
                      <span class="tome-sr-only">(not reached yet)</span>
                      <!-- Opens on hover and focus: the chapter can still be chosen. -->
                      <svg class="tome-menu-icon tome-lock" viewBox="0 0 24 24" aria-hidden="true">
                        <rect x="5" y="11" width="14" height="10" rx="1.5" />
                        <path class="tome-lock-closed" d="M8 11V7a4 4 0 0 1 8 0v4" />
                        <path class="tome-lock-open" d="M8 11V7a4 4 0 0 1 7.8-1.3" />
                      </svg>
                    {/if}
                  </button>
                </li>
              {/if}
            {/if}
          {/each}
        </ol>
      {:else if typeof view === 'object'}
        {@const destination = view.confirmJump}
        {@const from = view.from}
        {@const target = bundle.chapters[destination.chapter]}
        <h2 class="tome-menu-heading">Skip ahead?</h2>
        <p>
          You haven't reached <strong>{target.title}</strong> yet. Jumping there will unlock references up to that
          chapter, which may contain spoilers.
        </p>
        <div class="tome-menu-actions">
          <button class="tome-button tome-button-primary" onclick={() => onjump(destination)}>
            Jump to {target.title}
          </button>
          <button class="tome-button" onclick={() => show(from)}>Cancel</button>
        </div>
      {:else if view === 'reset'}
        <h2 class="tome-menu-heading">Reset reading progress?</h2>
        <p>
          The book starts again from the beginning, and references lock again until you reach their chapters. Your
          settings stay as they are. This can't be undone.
        </p>
        <label class="tome-option">
          <input type="checkbox" bind:checked={deleteAnnotations} />
          Also delete bookmarks and highlights
        </label>
        <div class="tome-menu-actions">
          <button class="tome-button tome-button-primary" onclick={reset}>Reset progress</button>
          <button class="tome-button" onclick={() => show('settings')}>Cancel</button>
        </div>
      {:else if view === 'bookmarks'}
        <h2 class="tome-menu-heading">Bookmarks</h2>
        {#if position}
          <button class="tome-button" onclick={() => progress.toggleBookmark(position)}>
            {currentBookmark ? 'Remove bookmark from this page' : 'Bookmark this page'}
          </button>
        {/if}
        {#if progress.save.bookmarks.length === 0}
          <p class="tome-menu-empty">No bookmarks yet. Use the bookmark button below the page, or press B, to add one.</p>
        {:else}
          <ul class="tome-menu-list">
            {#each progress.save.bookmarks as bookmark (bookmark.id)}
              <li class="tome-menu-record">
                <input
                  class="tome-input"
                  aria-label="Bookmark name"
                  value={bookmark.label}
                  onchange={(e) => progress.renameBookmark(bookmark.id, e.currentTarget.value)}
                />
                <span class="tome-menu-meta">{pageLabel(bookmark)}</span>
                <span class="tome-menu-record-actions">
                  <button class="tome-link-button" onclick={() => jumpTo(resolvePosition(bundle, bookmark), 'bookmarks')}>
                    Go
                  </button>
                  <button class="tome-link-button" onclick={() => progress.removeBookmark(bookmark.id)}>Remove</button>
                </span>
              </li>
            {/each}
          </ul>
        {/if}
      {:else if view === 'highlights'}
        <h2 class="tome-menu-heading">Highlights</h2>
        {#if highlights.length === 0}
          <p class="tome-menu-empty">No highlights yet. Select text on a page, then choose Highlight or the highlighter below the page (or press H).</p>
        {:else}
          <ul class="tome-menu-list">
            {#each highlights as highlight (highlight.id)}
              <li class="tome-menu-record">
                <blockquote class="tome-menu-quote">{highlight.text}</blockquote>
                <span class="tome-menu-meta">{pageLabel(highlight)}</span>
                <textarea
                  class="tome-input"
                  rows="2"
                  placeholder="Add a note"
                  aria-label="Note"
                  value={highlight.note}
                  onchange={(e) => progress.setHighlightNote(highlight.id, e.currentTarget.value)}
                ></textarea>
                <span class="tome-menu-record-actions">
                  <button class="tome-link-button" onclick={() => jumpTo(resolvePosition(bundle, highlight), 'highlights')}>
                    Go
                  </button>
                  <button class="tome-link-button" onclick={() => progress.removeHighlight(highlight.id)}>Remove</button>
                </span>
              </li>
            {/each}
          </ul>
        {/if}
      {:else if view === 'settings'}
        <h2 class="tome-menu-heading">Settings</h2>

        {#if desktop}
          <fieldset class="tome-fieldset">
            <legend>Display</legend>
            <label class="tome-option">
              <input
                type="checkbox"
                checked={desktop.fullscreen}
                onchange={(e) => desktop?.setFullscreen(e.currentTarget.checked)}
              />
              Full screen (F11)
            </label>
          </fieldset>
        {/if}

        <fieldset class="tome-fieldset">
          <legend>Text</legend>
          {#each Object.entries(BODY_FONTS) as [font, { label, stack }] (font)}
            <label class="tome-option" style:font-family={stack ?? 'var(--tome-font-body)'}>
              <input
                type="radio"
                name="body-font"
                checked={settings.body_font === font}
                onchange={() => progress.updateSettings({ body_font: font as BodyFont })}
              />
              {label}
            </label>
          {/each}
          <label class="tome-range">
            <span>Text size <output>{Math.round(settings.font_scale * 100)}%</output></span>
            <input
              type="range"
              min={FONT_SCALE_RANGE.min}
              max={FONT_SCALE_RANGE.max}
              step={FONT_SCALE_RANGE.step}
              value={settings.font_scale}
              oninput={(e) => progress.updateSettings({ font_scale: Number(e.currentTarget.value) })}
            />
          </label>
        </fieldset>

        <fieldset class="tome-fieldset">
          <legend>Effects</legend>
          <label class="tome-option">
            <input
              type="checkbox"
              checked={settings.special_text}
              onchange={(e) => progress.updateSettings({ special_text: e.currentTarget.checked })}
            />
            Special text and animations
          </label>
          <label class="tome-option">
            <input
              type="checkbox"
              checked={settings.accents}
              onchange={(e) => progress.updateSettings({ accents: e.currentTarget.checked })}
            />
            Color accents
          </label>
        </fieldset>

        <fieldset class="tome-fieldset">
          <legend>Sound</legend>
          {#each AUDIO_CHANNELS as { channel, label } (channel)}
            <div class="tome-volume">
              <label class="tome-range">
                <span>{label} <output>{Math.round(settings.audio[channel].volume * 100)}%</output></span>
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.05"
                  value={settings.audio[channel].volume}
                  disabled={settings.audio[channel].muted}
                  oninput={(e) => setVolume(channel, Number(e.currentTarget.value))}
                />
              </label>
              <label class="tome-option">
                <input
                  type="checkbox"
                  checked={settings.audio[channel].muted}
                  onchange={(e) => setMuted(channel, e.currentTarget.checked)}
                />
                Mute
              </label>
            </div>
          {/each}
        </fieldset>

        <fieldset class="tome-fieldset">
          <legend>Reading</legend>
          <label class="tome-option">
            <input
              type="checkbox"
              checked={settings.narration_highlight}
              onchange={(e) => progress.updateSettings({ narration_highlight: e.currentTarget.checked })}
            />
            Highlight text as it's narrated
          </label>
          <label class="tome-option">
            <input
              type="checkbox"
              checked={settings.auto_advance}
              onchange={(e) => progress.updateSettings({ auto_advance: e.currentTarget.checked })}
            />
            Advance automatically
          </label>
          <label class="tome-range">
            <span>Every <output>{settings.auto_interval_s} s</output></span>
            <input
              type="range"
              min="2"
              max="30"
              step="1"
              value={settings.auto_interval_s}
              disabled={!settings.auto_advance}
              oninput={(e) => progress.updateSettings({ auto_interval_s: Number(e.currentTarget.value) })}
            />
          </label>
          <label class="tome-option">
            <input
              type="checkbox"
              checked={settings.already_read}
              onchange={(e) => progress.updateSettings({ already_read: e.currentTarget.checked })}
            />
            I've already read this book (unlocks every reference)
          </label>
        </fieldset>

        <fieldset class="tome-fieldset">
          <legend>Progress</legend>
          <button class="tome-button" onclick={() => show('reset')}>Reset reading progress…</button>
        </fieldset>
      {/if}
    {/if}
  </div>
</dialog>
