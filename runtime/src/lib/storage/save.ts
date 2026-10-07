import type { Bundle } from '../bundle/types'
import { arriveBackward, type Position, type ReaderState } from '../reader/navigation'
import { emptyPace, type Pace } from '../reader/pace'
import type { StorageAdapter } from './adapter'

const SAVE_VERSION = 1

/** A page, by stable chapter ID. The content hash detects chapters edited since. */
export interface PageRef {
  chapter_id: string
  page: number
  content_hash: string
}

export interface Bookmark extends PageRef {
  id: string
  label: string
  created_at: string
}

/** A character offset into a block's visible text. */
export interface TextAnchor {
  block_id: string
  offset: number
}

export interface Highlight extends PageRef {
  id: string
  start: TextAnchor
  end: TextAnchor
  /** The highlighted text; if it no longer matches after an update, the highlight is hidden. */
  text: string
  note: string
  created_at: string
}

export type AudioChannelName = 'master' | 'music' | 'ambience' | 'sfx' | 'voice'

export interface ChannelSetting {
  volume: number
  muted: boolean
}

export type BodyFont = 'book' | 'serif' | 'sans' | 'hyperlegible'

export type TextWidth = 'narrow' | 'book' | 'wide'

export interface Settings {
  body_font: BodyFont
  /** Multiplies the theme's body text size. */
  font_scale: number
  /** Multiplies the theme's line height for body text. */
  line_spacing: number
  text_width: TextWidth
  /** Maximum-contrast text and paper in the theme's polarity; no textures or tints. */
  high_contrast: boolean
  accents: boolean
  /** Special typography plus entrance and resting animations. */
  special_text: boolean
  audio: Record<AudioChannelName, ChannelSetting>
  /** Show captions for music, ambience and sound effects. */
  captions: boolean
  /** Highlight the words (or paragraph) being narrated. */
  narration_highlight: boolean
  /** Show the estimated time left in the chapter in the status bar. */
  time_left: boolean
  auto_advance: boolean
  auto_interval_s: number
  /** Unlocks every reference and removes spoiler warnings. */
  already_read: boolean
}

/** Everything remembered about the reader, keyed by stable IDs so it survives book updates. */
export interface SaveState {
  version: typeof SAVE_VERSION
  /** Null until the reader has started. */
  position: PageRef | null
  /** Drives spoiler gating for references. */
  furthest_chapter_id: string | null
  bookmarks: Bookmark[]
  highlights: Highlight[]
  settings: Settings
  /** The reader's measured reading pace, for estimating time left. */
  pace: Pace
}

export const FONT_SCALE_RANGE = { min: 0.8, max: 1.6, step: 0.1 }
export const LINE_SPACING_RANGE = { min: 0.8, max: 1.6, step: 0.1 }

export function defaultSettings(prefersReducedMotion: boolean, prefersMoreContrast = false): Settings {
  const channel = (volume: number): ChannelSetting => ({ volume, muted: false })
  return {
    body_font: 'book',
    font_scale: 1,
    line_spacing: 1,
    text_width: 'book',
    high_contrast: prefersMoreContrast,
    accents: true,
    special_text: !prefersReducedMotion,
    audio: { master: channel(1), music: channel(0.8), ambience: channel(0.8), sfx: channel(1), voice: channel(1) },
    captions: false,
    narration_highlight: true,
    time_left: true,
    auto_advance: false,
    auto_interval_s: 8,
    already_read: false,
  }
}

export function emptySave(settings: Settings): SaveState {
  return {
    version: SAVE_VERSION,
    position: null,
    furthest_chapter_id: null,
    bookmarks: [],
    highlights: [],
    settings,
    pace: emptyPace(),
  }
}

function saveKey(bundle: Bundle): string {
  return `tome:${bundle.book.id}:save`
}

/** Read the save, filling anything missing (older saves, new settings) from the defaults. */
export async function readSave(storage: StorageAdapter, bundle: Bundle, defaults: Settings): Promise<SaveState> {
  const raw = await storage.load(saveKey(bundle))
  if (!raw) return emptySave(defaults)
  try {
    const save = JSON.parse(raw) as Partial<SaveState>
    if (save.version !== SAVE_VERSION) return emptySave(defaults)
    return {
      version: SAVE_VERSION,
      position: save.position ?? null,
      furthest_chapter_id: save.furthest_chapter_id ?? null,
      bookmarks: save.bookmarks ?? [],
      highlights: save.highlights ?? [],
      settings: {
        ...defaults,
        ...save.settings,
        audio: { ...defaults.audio, ...save.settings?.audio },
      },
      pace: save.pace ?? emptyPace(),
    }
  } catch {
    return emptySave(defaults)
  }
}

export async function writeSave(storage: StorageAdapter, bundle: Bundle, save: SaveState): Promise<void> {
  await storage.save(saveKey(bundle), JSON.stringify(save))
}

export function pageRef(bundle: Bundle, { chapter, page }: Position): PageRef {
  const { id, content_hash } = bundle.chapters[chapter]
  return { chapter_id: id, page, content_hash }
}

/**
 * Map a saved page onto the current bundle. If the chapter's content changed since it was
 * saved, fall back to the start of that chapter; if the chapter is gone, the book start.
 * While previewing, an author editing a chapter keeps their page instead.
 */
export function resolvePosition(bundle: Bundle, ref: PageRef, { keepPageOnEdit = false } = {}): Position {
  const chapter = bundle.chapters.findIndex((c) => c.id === ref.chapter_id)
  if (chapter === -1) return { chapter: 0, page: 0 }

  const { content_hash, pages } = bundle.chapters[chapter]
  if (content_hash !== ref.content_hash && !keepPageOnEdit) return { chapter, page: 0 }
  return { chapter, page: Math.min(Math.max(ref.page, 0), pages.length - 1) }
}

/** Index of the furthest chapter reached, or -1 before reading starts. */
export function furthestIndex(bundle: Bundle, save: SaveState): number {
  return bundle.chapters.findIndex((c) => c.id === save.furthest_chapter_id)
}

/** The chapter whose references are unlocked: everything once the book has been read. */
export function unlockedChapter(bundle: Bundle, save: SaveState): number {
  return save.settings.already_read ? bundle.chapters.length - 1 : Math.max(0, furthestIndex(bundle, save))
}

/** Move to a position, advancing the furthest chapter if needed (it never moves back). */
export function withPosition(bundle: Bundle, save: SaveState, position: Position): SaveState {
  const furthest = Math.max(furthestIndex(bundle, save), position.chapter)
  return { ...save, position: pageRef(bundle, position), furthest_chapter_id: bundle.chapters[furthest].id }
}

/** Narration plays only if the reader can hear it: neither it nor all sound is muted or at zero. */
export function narrationAudible(settings: Settings): boolean {
  const { master, voice } = settings.audio
  return !master.muted && master.volume > 0 && !voice.muted && voice.volume > 0
}

/** A chapter the reader hasn't reached: jumping there unlocks references that may spoil it. */
export function chapterLocked(bundle: Bundle, save: SaveState, chapter: number): boolean {
  return chapter > unlockedChapter(bundle, save)
}

/** Jumping to a page: one in a chapter already reached is shown fully revealed. */
export function arriveByJump(bundle: Bundle, save: SaveState, position: Position): ReaderState {
  return position.chapter <= furthestIndex(bundle, save)
    ? arriveBackward(bundle, position)
    : { position, revealed: 0, direction: 'forward' }
}

/** Highlights whose text still matches; the others stay saved but aren't shown. */
export function isHighlightCurrent(bundle: Bundle, highlight: Highlight): boolean {
  const chapter = bundle.chapters.find((c) => c.id === highlight.chapter_id)
  return chapter?.content_hash === highlight.content_hash
}
