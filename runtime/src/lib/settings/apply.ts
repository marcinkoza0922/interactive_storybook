import type { AudioEngine } from '../audio/engine'
import type { AudioChannelName, BodyFont, Settings, TextWidth } from '../storage/save'

export const BODY_FONTS: Record<BodyFont, { label: string; stack: string | null }> = {
  book: { label: 'Book default', stack: null },
  serif: { label: 'Serif', stack: "Georgia, 'Times New Roman', serif" },
  sans: { label: 'Sans-serif', stack: "system-ui, 'Segoe UI', Roboto, sans-serif" },
  hyperlegible: { label: 'Atkinson Hyperlegible', stack: "'Atkinson Hyperlegible', system-ui, sans-serif" },
}

/** Text width, as a multiple of the theme's page width. */
export const TEXT_WIDTHS: Record<TextWidth, { label: string; scale: number }> = {
  narrow: { label: 'Narrow', scale: 0.8 },
  book: { label: 'Book default', scale: 1 },
  wide: { label: 'Wide', scale: 1.25 },
}

export const AUDIO_CHANNELS: { channel: AudioChannelName; label: string }[] = [
  { channel: 'master', label: 'Master' },
  { channel: 'music', label: 'Music' },
  { channel: 'ambience', label: 'Ambience' },
  { channel: 'sfx', label: 'Sound effects' },
  { channel: 'voice', label: 'Narration' },
]

/** Reflect reader settings on the root element, where the runtime CSS and themes read them. */
export function applyDocumentSettings(root: HTMLElement, settings: Settings): void {
  root.dataset.specialText = settings.special_text ? 'on' : 'off'
  root.dataset.accents = settings.accents ? 'on' : 'off'
  if (settings.high_contrast) root.dataset.contrast = 'high'
  else delete root.dataset.contrast
  root.style.setProperty('--tome-font-scale', String(settings.font_scale))
  root.style.setProperty('--tome-line-spacing', String(settings.line_spacing))
  root.style.setProperty('--tome-text-width-scale', String(TEXT_WIDTHS[settings.text_width]?.scale ?? 1))
  const stack = BODY_FONTS[settings.body_font]?.stack
  if (stack) root.style.setProperty('--tome-reader-font', stack)
  else root.style.removeProperty('--tome-reader-font')
}

export function applyAudioSettings(engine: AudioEngine, settings: Settings): void {
  for (const { channel } of AUDIO_CHANNELS) {
    const { volume, muted } = settings.audio[channel]
    engine.setChannelVolume(channel, muted ? 0 : volume)
  }
}
