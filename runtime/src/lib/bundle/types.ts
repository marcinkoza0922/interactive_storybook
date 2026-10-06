// The book bundle: the contract between the `tome` compiler and this runtime.
// Keys are snake_case because the compiler (Rust/serde) emits them that way.

/** The bundle schema version this runtime understands. Bump on breaking changes. */
export const SUPPORTED_SCHEMA_VERSION = 1

export interface Bundle {
  bundle_schema_version: number
  book: BookMeta
  chapters: Chapter[]
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
  /** Pre-paginated by the compiler. Never empty. */
  pages: Page[]
}

export interface Page {
  blocks: Block[]
}

export interface Block {
  /** Unique within the bundle. */
  id: string
  /** Rendered HTML for the block (a paragraph, heading, etc.). */
  html: string
  /** Absent: visible as soon as the page is shown. */
  reveal?: Reveal
}

export type EntranceEffect = 'fade'

export interface Reveal {
  /** 1-based step at which this block appears. Blocks sharing a step appear together. */
  step: number
  effect: EntranceEffect
  duration_ms?: number
}
