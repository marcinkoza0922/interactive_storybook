// The book bundle: the contract between the `tome` compiler and this runtime.
// Keys are snake_case because the compiler (Rust/serde) emits them that way.

/** The bundle schema version this runtime understands. Bump on breaking changes. */
export const SUPPORTED_SCHEMA_VERSION = 1

export interface Bundle {
  bundle_schema_version: number
  book: BookMeta
  audio?: AudioConfig
  illustrations?: IllustrationConfig
  chapters: Chapter[]
  /** The chapter index as the author arranged it (from contents.toml). Absent: every chapter, in order. */
  contents?: ContentsEntry[]
  references?: Reference[]
  theme?: Theme
}

export type ContentsEntry =
  | { kind: 'chapter'; id: string; /** Overrides the chapter title in the index. */ title?: string }
  | { kind: 'heading'; title: string }

export interface AudioConfig {
  /** Delay before restoring a page's music and ambience after navigating back to it. */
  restore_delay_ms?: number
}

export interface IllustrationConfig {
  /** On narrow screens, how long a newly introduced track illustration shows before the text. */
  linger_ms?: number
}

export interface BookMeta {
  /** Stable identifier; scopes saved state. */
  id: string
  title: string
  author: string
  /** BCP 47 tag, e.g. "en". */
  language: string
}

export interface Chapter {
  /** Stable identifier used by saves, reference gating and theme overrides. */
  id: string
  title: string
  /** Changes whenever the chapter's content changes; invalidates saved page positions. */
  content_hash: string
  /** Art shown above the chapter title. */
  header_image?: ImageRef
  /** Pre-paginated by the compiler. Never empty. */
  pages: Page[]
}

export interface Page {
  blocks: Block[]
  /**
   * Sets the illustration track from this page on: an image, or null to clear it.
   * Absent: the previous page's track illustration carries over.
   */
  illustration?: ImageRef | null
  /** A named paper from the theme, for this page only. */
  paper?: string
  /**
   * IDs of references mentioned on this page, in order of first mention. Matching names and
   * aliases against the text (and gating aliases by chapter) is the compiler's job.
   */
  references?: string[]
}

export interface Block {
  /** Unique within the bundle. */
  id: string
  /** Rendered HTML for the block (a paragraph, heading, etc.). */
  html: string
  /** Absent: visible as soon as the page is shown. */
  reveal?: Reveal
  /** Audio cues that fire when this block becomes visible, in order. */
  cues?: Cue[]
}

export type EntranceEffect = 'fade' | 'slide' | 'typewriter'

/**
 * Resting effects (pulse, breathe, tremble, gradient, wave) and named special styles aren't
 * fields: they are `data-tome-rest` and `data-tome-style` attributes on elements in the block
 * HTML, with optional parameters as CSS custom properties (`--tome-rest-duration`, ...).
 */
export interface Reveal {
  /** 1-based step at which this block appears. Blocks sharing a step appear together. */
  step: number
  effect: EntranceEffect
  /** For typewriter: the time to type the whole block (default: 35ms per character). */
  duration_ms?: number
  delay_ms?: number
  /** A CSS easing function, e.g. "ease-in-out" or "cubic-bezier(...)". */
  easing?: string
}

/** Paths in cues are relative to the bundle's book.json. Volumes are 0–1, default 1. */
export type Cue =
  | { kind: 'music'; src: string; volume?: number; fade_ms?: number; delay_ms?: number }
  | { kind: 'music_stop'; fade_ms?: number; delay_ms?: number }
  | { kind: 'ambient'; id: string; src: string; volume?: number; fade_ms?: number; delay_ms?: number }
  | { kind: 'ambient_stop'; id: string; fade_ms?: number; delay_ms?: number }
  | { kind: 'sfx'; src: string; volume?: number; delay_ms?: number }

export interface Reference {
  id: string
  /** In chapter order. The reference is hidden until its first section unlocks. */
  sections: ReferenceSection[]
}

export interface ReferenceSection {
  /** Chapter ID from which this section is visible (once the reader has reached it). */
  from: string
  /** `replace` (default) supersedes earlier sections; `append` adds to them. */
  mode?: 'replace' | 'append'
  /** The displayed name from this section on. Gated like the text, so names can't leak. */
  title?: string
  /** Rendered HTML. */
  html: string
  image?: ImageRef
}

export interface ImageRef {
  /** Relative to the bundle's book.json. */
  src: string
  alt: string
}

/** Asset paths are relative to book.json. */
export interface Theme {
  /**
   * Values for the runtime's CSS custom properties, named without the `--tome-` prefix,
   * e.g. `{ "accent": "#8a3b2e", "font-body": "Lora, serif" }`. Named gradients for the
   * gradient effect are tokens too, e.g. `"gradient-dawn": "linear-gradient(...)"`.
   */
  tokens?: Record<string, string>
  fonts?: FontFile[]
  /** Named special styles: CSS declarations for `[data-tome-style="<name>"]`. */
  styles?: Record<string, Record<string, string>>
  /** Named papers that pages can use instead of the book's own. */
  papers?: Record<string, Paper>
  backgrounds?: Backgrounds
  decoration?: Decoration
  landing?: LandingTheme
  /** A stylesheet loaded after everything else. */
  custom_css?: string
  /** Partial themes applied from a chapter onward, in order, following the reader's current chapter. */
  overrides?: ThemeOverride[]
}

export interface FontFile {
  family: string
  src: string
  weight?: string
  style?: string
}

/** A sheet of paper. Unset fields keep the book's paper. */
export interface Paper {
  color?: string
  /** A tiled image, or null for none. */
  texture?: string | null
  /** Generated grain, 0 (none) to 1. */
  grain?: number
}

/** An image, GIF or video, a plain color, or a color behind a partly transparent image. */
export interface Background {
  src?: string
  color?: string
  /** Inferred from the extension when absent. */
  kind?: 'image' | 'animated' | 'video'
  /** Still frame shown instead of a video or GIF when motion is turned off. */
  poster?: string
  fit?: 'cover' | 'contain'
  position?: string
  opacity?: number
}

export interface Backgrounds {
  landing?: Background | null
  reading?: Background | null
}

export interface Decoration {
  /** Tiled image behind the text. */
  page_texture?: string | null
  /** A border image around the page; `slice` in image pixels, `width` as a CSS length. */
  page_frame?: { src: string; slice: number; width: string; repeat?: 'stretch' | 'round' | 'repeat' } | null
  /** Shown above chapter titles of chapters without their own header art. */
  chapter_ornament?: string | null
  drop_caps?: boolean
  /** Generated grain on the paper, 0 (none) to 1. */
  paper_grain?: number
}

export interface LandingTheme {
  cover?: ImageRef
  /** Replaces the text title; its alt text should be the title. */
  title_image?: ImageRef
  layout?: 'centered' | 'split'
  menu?: 'stacked' | 'inline'
  /** Music for the title screen, from the first interaction with it. */
  music?: string
}

export interface ThemeOverride {
  from: string
  tokens?: Record<string, string>
  styles?: Record<string, Record<string, string>>
  backgrounds?: Backgrounds
  decoration?: Decoration
}
