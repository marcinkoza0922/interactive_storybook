<script lang="ts">
  import { onDestroy } from 'svelte'
  import { backgroundKind, themeCss, type EffectiveTheme } from '../theme/theme'

  interface Props {
    theme: EffectiveTheme
    screen: 'landing' | 'reading'
    /** False when the reader turned off special text or prefers reduced motion. */
    motion: boolean
    assetUrl: (src: string) => string
  }

  let { theme, screen, motion, assetUrl }: Props = $props()

  const background = $derived(theme.backgrounds[screen] ?? null)
  const kind = $derived(background ? backgroundKind(background) : null)
  const customCss = $derived(theme.custom_css)

  // The generated stylesheet and the author's custom CSS go at the end of <head>, after the
  // runtime's own styles, so they win at equal specificity. Custom CSS comes last of all.
  const style = document.createElement('style')
  style.id = 'tome-theme'
  document.head.append(style)
  onDestroy(() => style.remove())

  $effect(() => {
    style.textContent = themeCss(theme, assetUrl)
  })

  $effect(() => {
    if (!customCss) return
    const link = document.createElement('link')
    link.rel = 'stylesheet'
    link.href = assetUrl(customCss)
    style.after(link)
    return () => link.remove()
  })

  // Decoration switches that CSS can't infer from token values.
  $effect(() => {
    const root = document.documentElement.dataset
    root.dropCaps = theme.decoration.drop_caps ? 'on' : 'off'
    root.chapterOrnament = theme.decoration.chapter_ornament ? 'on' : 'off'
    root.pageFrame = theme.decoration.page_frame ? 'on' : 'off'
  })
</script>

{#if background}
  {#key background.src}
    <div
      class="tome-backdrop"
      aria-hidden="true"
      style:opacity={background.opacity}
      style:--tome-backdrop-fit={background.fit}
      style:--tome-backdrop-position={background.position}
    >
      {#if kind === 'video' && motion}
        <video
          src={assetUrl(background.src)}
          poster={background.poster ? assetUrl(background.poster) : undefined}
          autoplay
          muted
          loop
          playsinline
        ></video>
      {:else if kind === 'image' || motion}
        <img src={assetUrl(background.src)} alt="" />
      {:else if background.poster}
        <!-- Motion is off: a still frame instead of the video or animation. -->
        <img src={assetUrl(background.poster)} alt="" />
      {/if}
    </div>
  {/key}
{/if}
