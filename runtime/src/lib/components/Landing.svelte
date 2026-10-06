<script lang="ts">
  import type { BookMeta, LandingTheme } from '../bundle/types'
  import { desktop } from '../desktop.svelte'

  interface Props {
    book: BookMeta
    theme: LandingTheme
    canContinue: boolean
    oncontinue: () => void
    onbegin: () => void
    assetUrl: (src: string) => string
  }

  let { book, theme, canContinue, oncontinue, onbegin, assetUrl }: Props = $props()
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
        <button class="tome-button tome-button-primary" onclick={oncontinue} autofocus>Continue</button>
        <button class="tome-button" onclick={onbegin}>Start from the beginning</button>
      {:else}
        <!-- svelte-ignore a11y_autofocus -->
        <button class="tome-button tome-button-primary" onclick={onbegin} autofocus>Begin</button>
      {/if}
      {#if desktop}
        <button class="tome-button" onclick={() => desktop?.quit()}>Exit</button>
      {/if}
    </nav>
  </div>
</section>
