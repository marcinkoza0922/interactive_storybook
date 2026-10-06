<script lang="ts">
  import type { Page } from '../bundle/types'

  interface Props {
    page: Page
    /** Shown above the first page of a chapter. */
    chapterTitle: string | null
    revealed: number
    entering: ReadonlySet<string>
    onentered: (blockId: string) => void
  }

  let { page, chapterTitle, revealed, entering, onentered }: Props = $props()
</script>

<article class="tome-page">
  {#if chapterTitle}
    <h1 class="tome-chapter-title">{chapterTitle}</h1>
  {/if}

  {#each page.blocks as block (block.id)}
    {@const visible = (block.reveal?.step ?? 0) <= revealed}
    <div
      class="tome-block"
      class:tome-hidden={!visible}
      class:tome-entering={entering.has(block.id)}
      data-block-id={block.id}
      data-effect={block.reveal?.effect}
      style:--tome-reveal-duration={block.reveal?.duration_ms ? `${block.reveal.duration_ms}ms` : undefined}
      onanimationend={(event) => event.target === event.currentTarget && onentered(block.id)}
    >
      {@html block.html}
    </div>
  {/each}
</article>
