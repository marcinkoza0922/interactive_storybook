import type { Bundle } from '../bundle/types'
import type { Position } from '../reader/navigation'
import type { StorageAdapter } from './adapter'

const SAVE_VERSION = 1

/** Reader progress, keyed by stable chapter IDs so it survives book updates. */
export interface SaveState {
  version: typeof SAVE_VERSION
  position: {
    chapter_id: string
    page: number
    /** The chapter's content hash when the position was saved. */
    content_hash: string
  }
  /** Drives spoiler gating for references. */
  furthest_chapter_id: string
}

function saveKey(bundle: Bundle): string {
  return `tome:${bundle.book.id}:save`
}

export async function readSave(storage: StorageAdapter, bundle: Bundle): Promise<SaveState | null> {
  const raw = await storage.load(saveKey(bundle))
  if (!raw) return null
  try {
    const save = JSON.parse(raw) as SaveState
    return save.version === SAVE_VERSION ? save : null
  } catch {
    return null
  }
}

export async function writeSave(storage: StorageAdapter, bundle: Bundle, save: SaveState): Promise<void> {
  await storage.save(saveKey(bundle), JSON.stringify(save))
}

/**
 * Map a saved position onto the current bundle. If the chapter's content changed since
 * the save, fall back to the start of that chapter; if the chapter is gone, the book start.
 */
export function resolvePosition(bundle: Bundle, save: SaveState): Position {
  const chapter = bundle.chapters.findIndex((c) => c.id === save.position.chapter_id)
  if (chapter === -1) return { chapter: 0, page: 0 }

  const { content_hash, pages } = bundle.chapters[chapter]
  if (content_hash !== save.position.content_hash) return { chapter, page: 0 }
  return { chapter, page: Math.min(Math.max(save.position.page, 0), pages.length - 1) }
}

/** Build the save for a new position, advancing the furthest chapter if needed. */
export function nextSave(bundle: Bundle, position: Position, previous: SaveState | null): SaveState {
  const chapter = bundle.chapters[position.chapter]
  const previousFurthest = previous
    ? bundle.chapters.findIndex((c) => c.id === previous.furthest_chapter_id)
    : -1
  const furthest = Math.max(previousFurthest, position.chapter)

  return {
    version: SAVE_VERSION,
    position: { chapter_id: chapter.id, page: position.page, content_hash: chapter.content_hash },
    furthest_chapter_id: bundle.chapters[furthest].id,
  }
}
