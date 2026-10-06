<script lang="ts">
  import type { VisibleEntry } from '../references/gating'

  interface Props {
    id: string
    /** Visible references on the current page. */
    entries: VisibleEntry[]
    /** The entry being read, if any. Stays open across page turns. */
    selected: VisibleEntry | null
    onselect: (id: string | null) => void
    onclose: () => void
    assetUrl: (src: string) => string
  }

  let { id, entries, selected, onselect, onclose, assetUrl }: Props = $props()
</script>

<aside {id} class="tome-sidebar" aria-label="References">
  <header class="tome-sidebar-header">
    {#if selected}
      <button class="tome-link-button" onclick={() => onselect(null)}>← On this page</button>
    {:else}
      <h2 class="tome-sidebar-heading">On this page</h2>
    {/if}
    <button class="tome-icon-button" onclick={onclose} aria-label="Close references">×</button>
  </header>

  {#if selected}
    <article class="tome-reference" aria-labelledby="tome-reference-title">
      <h3 id="tome-reference-title" class="tome-reference-title">{selected.title}</h3>
      {#if selected.image}
        <img class="tome-reference-image" src={assetUrl(selected.image.src)} alt={selected.image.alt} />
      {/if}
      {#each selected.sections as html, i (i)}
        <div class="tome-reference-section">{@html html}</div>
      {/each}
    </article>
  {:else if entries.length > 0}
    <ul class="tome-reference-list">
      {#each entries as entry (entry.id)}
        <li><button class="tome-reference-link" onclick={() => onselect(entry.id)}>{entry.title}</button></li>
      {/each}
    </ul>
  {:else}
    <p class="tome-sidebar-empty">Nothing to look up on this page.</p>
  {/if}
</aside>
