<script lang="ts">
  import { fade } from 'svelte/transition'
  import type { Captions } from '../audio/captions.svelte'

  let { captions }: { captions: Captions } = $props()

  /** What's still sounding, leaving out what a fresh caption already says. */
  const lingering = $derived(captions.soundscape.filter((text) => !captions.current.some((c) => c.text === text)))
</script>

<div class="tome-captions">
  <!-- Only new sounds are announced; the soundscape is there to read, not to interrupt. -->
  <div class="tome-captions-current" aria-live="polite">
    {#each captions.current as caption (caption.id)}
      <p class="tome-caption" transition:fade={{ duration: 300 }}>{caption.text}</p>
    {/each}
  </div>
  {#if lingering.length > 0}
    <p class="tome-captions-soundscape">{lingering.join(' · ')}</p>
  {/if}
</div>
