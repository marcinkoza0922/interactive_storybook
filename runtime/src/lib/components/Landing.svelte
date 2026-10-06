<script lang="ts">
  import type { BookMeta } from '../bundle/types'

  interface Props {
    book: BookMeta
    canContinue: boolean
    oncontinue: () => void
    onbegin: () => void
  }

  let { book, canContinue, oncontinue, onbegin }: Props = $props()
</script>

<!-- Starting from a button doubles as the user gesture browsers require before audio. -->
<section class="tome-landing">
  <h1 class="tome-landing-title">{book.title}</h1>
  <p class="tome-landing-author">{book.author}</p>

  <nav class="tome-landing-menu">
    {#if canContinue}
      <!-- svelte-ignore a11y_autofocus -->
      <button class="tome-button tome-button-primary" onclick={oncontinue} autofocus>Continue</button>
      <button class="tome-button" onclick={onbegin}>Start from the beginning</button>
    {:else}
      <!-- svelte-ignore a11y_autofocus -->
      <button class="tome-button tome-button-primary" onclick={onbegin} autofocus>Begin</button>
    {/if}
  </nav>
</section>
