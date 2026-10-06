<script lang="ts">
  import { tick } from 'svelte'
  import type { Bundle } from '../bundle/types'
  import type { Position } from '../reader/navigation'
  import { AUDIO_CHANNELS, BODY_FONTS } from '../settings/apply'
  import type { Progress } from '../state/progress.svelte'
  import {
    FONT_SCALE_RANGE,
    isHighlightCurrent,
    jumpRevealsSpoilers,
    resolvePosition,
    type BodyFont,
    type PageRef,
  } from '../storage/save'

  interface Props {
    bundle: Bundle
    progress: Progress
    position: Position
    onclose: () => void
    /** Go to a page; `seen` pages are shown fully revealed. */
    onjump: (position: Position) => void
    onreferences: () => void
    ontitle: () => void
  }

  let { bundle, progress, position, onclose, onjump, onreferences, ontitle }: Props = $props()

  type View = 'main' | 'chapters' | 'bookmarks' | 'highlights' | 'settings' | { confirmJump: number }
  let view = $state<View>('main')
  let dialog: HTMLDialogElement

  const settings = $derived(progress.settings)
  const currentBookmark = $derived(progress.bookmarkAt(position))
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

  function chooseChapter(chapter: number) {
    if (jumpRevealsSpoilers(bundle, progress.save, chapter)) show({ confirmJump: chapter })
    else onjump({ chapter, page: 0 })
  }

  function setVolume(channel: (typeof AUDIO_CHANNELS)[number]['channel'], volume: number) {
    progress.updateSettings({ audio: { ...settings.audio, [channel]: { ...settings.audio[channel], volume } } })
  }

  function setMuted(channel: (typeof AUDIO_CHANNELS)[number]['channel'], muted: boolean) {
    progress.updateSettings({ audio: { ...settings.audio, [channel]: { ...settings.audio[channel], muted } } })
  }
</script>

<dialog class="tome-menu" bind:this={dialog} onclose={onclose} aria-label="Menu">
  <div class="tome-menu-view">
    {#if view === 'main'}
      <h2 class="tome-menu-heading">{bundle.book.title}</h2>
      <nav class="tome-menu-list">
        <button class="tome-menu-item" onclick={() => dialog.close()}>Resume</button>
        <button class="tome-menu-item" onclick={() => show('chapters')}>Chapters</button>
        <button class="tome-menu-item" onclick={() => show('bookmarks')}>
          Bookmarks <span class="tome-menu-count">{progress.save.bookmarks.length || ''}</span>
        </button>
        <button class="tome-menu-item" onclick={() => show('highlights')}>
          Highlights <span class="tome-menu-count">{highlights.length || ''}</span>
        </button>
        <button class="tome-menu-item" onclick={onreferences}>References</button>
        <button class="tome-menu-item" onclick={() => show('settings')}>Settings</button>
        <button class="tome-menu-item" onclick={ontitle}>Title screen</button>
      </nav>
    {:else}
      <button class="tome-link-button tome-menu-back" onclick={() => show(typeof view === 'object' ? 'chapters' : 'main')}>
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
                <li>
                  <button
                    class="tome-menu-item"
                    aria-current={chapter === position.chapter ? 'true' : undefined}
                    onclick={() => chooseChapter(chapter)}
                  >
                    {chapterTitle(entry.id, entry.title)}
                  </button>
                </li>
              {/if}
            {/if}
          {/each}
        </ol>
      {:else if typeof view === 'object'}
        {@const targetIndex = view.confirmJump}
        {@const target = bundle.chapters[targetIndex]}
        <h2 class="tome-menu-heading">Skip ahead?</h2>
        <p>
          Jumping to <strong>{target.title}</strong> will unlock references up to that chapter, which may contain
          spoilers.
        </p>
        <div class="tome-menu-actions">
          <button class="tome-button tome-button-primary" onclick={() => onjump({ chapter: targetIndex, page: 0 })}>
            Jump to {target.title}
          </button>
          <button class="tome-button" onclick={() => show('chapters')}>Cancel</button>
        </div>
      {:else if view === 'bookmarks'}
        <h2 class="tome-menu-heading">Bookmarks</h2>
        <button class="tome-button" onclick={() => progress.toggleBookmark(position)}>
          {currentBookmark ? 'Remove bookmark from this page' : 'Bookmark this page'}
        </button>
        {#if progress.save.bookmarks.length === 0}
          <p class="tome-menu-empty">No bookmarks yet. Press B on any page to add one.</p>
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
                  <button class="tome-link-button" onclick={() => onjump(resolvePosition(bundle, bookmark))}>Go</button>
                  <button class="tome-link-button" onclick={() => progress.removeBookmark(bookmark.id)}>Remove</button>
                </span>
              </li>
            {/each}
          </ul>
        {/if}
      {:else if view === 'highlights'}
        <h2 class="tome-menu-heading">Highlights</h2>
        {#if highlights.length === 0}
          <p class="tome-menu-empty">No highlights yet. Select text on a page, then choose Highlight (or press H).</p>
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
                  <button class="tome-link-button" onclick={() => onjump(resolvePosition(bundle, highlight))}>Go</button>
                  <button class="tome-link-button" onclick={() => progress.removeHighlight(highlight.id)}>Remove</button>
                </span>
              </li>
            {/each}
          </ul>
        {/if}
      {:else if view === 'settings'}
        <h2 class="tome-menu-heading">Settings</h2>

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
      {/if}
    {/if}
  </div>
</dialog>
