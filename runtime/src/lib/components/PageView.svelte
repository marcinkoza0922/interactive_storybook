<script lang="ts">
  import type { ImageRef, Page } from '../bundle/types'

  interface Props {
    page: Page
    /** Shown above the first page of a chapter. */
    chapterTitle: string | null
    /** Header art, shown above the title on a chapter's first page. */
    chapterImage?: ImageRef
    /** Fade the whole page in, e.g. after the illustration track lingered. */
    fadeIn: boolean
    revealed: number
    entering: ReadonlySet<string>
    onentered: (blockId: string) => void
    assetUrl: (src: string) => string
  }

  let { page, chapterTitle, chapterImage, fadeIn, revealed, entering, onentered, assetUrl }: Props = $props()
</script>

<article class="tome-page" class:tome-page-fade-in={fadeIn}>
  {#if chapterImage}
    <img class="tome-chapter-image" src={assetUrl(chapterImage.src)} alt={chapterImage.alt} />
  {/if}
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
