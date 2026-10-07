<script lang="ts">
  import { BODY_FONTS, TEXT_WIDTHS } from '../settings/apply'
  import type { Progress } from '../state/progress.svelte'
  import { FONT_SCALE_RANGE, LINE_SPACING_RANGE, type BodyFont, type TextWidth } from '../storage/save'

  let { progress }: { progress: Progress } = $props()

  const settings = $derived(progress.settings)
</script>

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
  <label class="tome-range">
    <span>Line spacing <output>{Math.round(settings.line_spacing * 100)}%</output></span>
    <input
      type="range"
      min={LINE_SPACING_RANGE.min}
      max={LINE_SPACING_RANGE.max}
      step={LINE_SPACING_RANGE.step}
      value={settings.line_spacing}
      oninput={(e) => progress.updateSettings({ line_spacing: Number(e.currentTarget.value) })}
    />
  </label>
</fieldset>

<fieldset class="tome-fieldset">
  <legend>Text width</legend>
  {#each Object.entries(TEXT_WIDTHS) as [width, { label }] (width)}
    <label class="tome-option">
      <input
        type="radio"
        name="text-width"
        checked={settings.text_width === width}
        onchange={() => progress.updateSettings({ text_width: width as TextWidth })}
      />
      {label}
    </label>
  {/each}
</fieldset>
