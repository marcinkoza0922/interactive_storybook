<script lang="ts">
  import type { Bundle, LandingTheme } from '../bundle/types'
  import { desktop } from '../desktop.svelte'
  import type { Position } from '../reader/navigation'
  import type { SearchMatch } from '../search/search'
  import type { Progress } from '../state/progress.svelte'
  import Menu from './Menu.svelte'

  interface Props {
    bundle: Bundle
    progress: Progress
    theme: LandingTheme
    canContinue: boolean
    oncontinue: () => void
    onbegin: () => void
    /** Start reading at a page chosen from the chapters, bookmarks or highlights. */
    onjump: (position: Position) => void
    /** Start reading at a search match. */
    onfind: (match: SearchMatch) => void
    assetUrl: (src: string) => string
  }

  let { bundle, progress, theme, canContinue, oncontinue, onbegin, onjump, onfind, assetUrl }: Props = $props()

  const book = $derived(bundle.book)
  let browsing = $state<'chapters' | 'bookmarks' | 'highlights' | 'search' | 'settings' | null>(null)
</script>

<!-- Starting from a button doubles as the user gesture browsers require before audio. -->
<section class="tome-landing" data-layout={theme.layout ?? 'centered'}>
  <div class="tome-landing-tools">
    <button
      class="tome-landing-tool"
      onclick={() => (browsing = 'search')}
      aria-haspopup="dialog"
      aria-label="Search"
      title="Search"
    >
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M17 10.5a6.5 6.5 0 1 1-13 0 6.5 6.5 0 1 1 13 0z M15.2 15.2L20.5 20.5" />
      </svg>
    </button>
    <button
      class="tome-landing-tool"
      onclick={() => (browsing = 'settings')}
      aria-haspopup="dialog"
      aria-label="Settings"
      title="Settings"
    >
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path
          d="M19.1 9.3L21.6 10.1L21.6 13.9L19.1 14.7L18.9 15.1L20.2 17.4L17.4 20.2L15.1 18.9L14.7 19.1L13.9 21.6L10.1 21.6L9.3 19.1L8.9 18.9L6.6 20.2L3.8 17.4L5.1 15.1L4.9 14.7L2.4 13.9L2.4 10.1L4.9 9.3L5.1 8.9L3.8 6.6L6.6 3.8L8.9 5.1L9.3 4.9L10.1 2.4L13.9 2.4L14.7 4.9L15.1 5.1L17.4 3.8L20.2 6.6L18.9 8.9z M15 12a3 3 0 1 1-6 0 3 3 0 1 1 6 0z"
        />
      </svg>
    </button>
  </div>

  {#if theme.cover}
    <img class="tome-landing-cover" src={assetUrl(theme.cover.src)} alt={theme.cover.alt} />
  {/if}

  <div class="tome-landing-body">
    <h1 class="tome-landing-title">
      {#if theme.title_image}
        <img class="tome-landing-title-image" src={assetUrl(theme.title_image.src)} alt={theme.title_image.alt || book.title} />
      {:else}
        {book.title}
      {/if}
    </h1>
    <p class="tome-landing-author">{book.author}</p>

    <nav class="tome-landing-menu" data-arrangement={theme.menu ?? 'stacked'}>
      {#if canContinue}
        <!-- svelte-ignore a11y_autofocus -->
        <button class="tome-button tome-button-primary" onclick={oncontinue} data-begins autofocus>Continue</button>
        <button class="tome-button" onclick={onbegin} data-begins>Start from the beginning</button>
      {:else}
        <!-- svelte-ignore a11y_autofocus -->
        <button class="tome-button tome-button-primary" onclick={onbegin} data-begins autofocus>Begin</button>
      {/if}
      <button class="tome-button" onclick={() => (browsing = 'chapters')} aria-haspopup="dialog">Go to chapter</button>
      <span class="tome-landing-shortcuts">
        <button class="tome-link-button tome-landing-shortcut" onclick={() => (browsing = 'bookmarks')} aria-haspopup="dialog">
          <svg class="tome-menu-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 3h12v18l-6-4.5L6 21z" /></svg>
          Bookmarks
        </button>
        <button class="tome-link-button tome-landing-shortcut" onclick={() => (browsing = 'highlights')} aria-haspopup="dialog">
          <svg class="tome-menu-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M15 4l5 5-9 9H6v-5z M4 21h16" /></svg>
          Highlights
        </button>
      </span>
      {#if desktop}
        <button class="tome-button" onclick={() => desktop?.quit()}>Exit</button>
      {/if}
    </nav>
  </div>
</section>

{#if browsing}
  <!-- Resetting progress from here just closes the menu: the title screen updates itself. -->
  <Menu
    {bundle}
    {progress}
    position={null}
    start={browsing}
    onclose={() => (browsing = null)}
    {onjump}
    {onfind}
    onreset={() => (browsing = null)}
  />
{/if}
