<script lang="ts">
  import type { Bundle, LandingTheme } from '../bundle/types'
  import { desktop } from '../desktop.svelte'
  import type { Position } from '../reader/navigation'
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
    assetUrl: (src: string) => string
  }

  let { bundle, progress, theme, canContinue, oncontinue, onbegin, onjump, assetUrl }: Props = $props()

  const book = $derived(bundle.book)
  let browsing = $state<'chapters' | 'bookmarks' | 'highlights' | null>(null)
</script>

<!-- Starting from a button doubles as the user gesture browsers require before audio. -->
<section class="tome-landing" data-layout={theme.layout ?? 'centered'}>
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
  <Menu {bundle} {progress} position={null} start={browsing} onclose={() => (browsing = null)} {onjump} />
{/if}
