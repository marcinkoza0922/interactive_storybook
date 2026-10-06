import { describe, expect, it } from 'vitest'
import type { Bundle } from '../bundle/types'
import { MemoryStorageAdapter } from './adapter'
import {
  defaultSettings,
  emptySave,
  jumpRevealsSpoilers,
  readSave,
  resolvePosition,
  unlockedChapter,
  withPosition,
  writeSave,
  type SaveState,
} from './save'

const pages = (n: number) => Array.from({ length: n }, (_, i) => ({ blocks: [{ id: `b${i}`, html: '' }] }))

const bundle: Bundle = {
  bundle_schema_version: 1,
  book: { id: 'test', title: 'Test', author: 'A', language: 'en' },
  chapters: [
    { id: 'one', title: 'One', content_hash: 'h1', pages: pages(3) },
    { id: 'two', title: 'Two', content_hash: 'h2', pages: pages(2) },
    { id: 'three', title: 'Three', content_hash: 'h3', pages: pages(1) },
    { id: 'four', title: 'Four', content_hash: 'h4', pages: pages(1) },
  ],
}

const defaults = defaultSettings(false)
const at = (chapter_id: string, page: number, content_hash: string) => ({ chapter_id, page, content_hash })
const reached = (chapter: number): SaveState => withPosition(bundle, emptySave(defaults), { chapter, page: 0 })

describe('resolvePosition', () => {
  it('restores an unchanged chapter exactly', () => {
    expect(resolvePosition(bundle, at('two', 1, 'h2'))).toEqual({ chapter: 1, page: 1 })
  })

  it('resets to the chapter start when the chapter content changed', () => {
    expect(resolvePosition(bundle, at('two', 1, 'stale'))).toEqual({ chapter: 1, page: 0 })
  })

  it('keeps the page while previewing edits', () => {
    expect(resolvePosition(bundle, at('two', 1, 'stale'), { keepPageOnEdit: true })).toEqual({ chapter: 1, page: 1 })
    expect(resolvePosition(bundle, at('two', 9, 'stale'), { keepPageOnEdit: true })).toEqual({ chapter: 1, page: 1 })
  })

  it('falls back to the book start when the chapter no longer exists', () => {
    expect(resolvePosition(bundle, at('gone', 1, 'h2'))).toEqual({ chapter: 0, page: 0 })
  })
})

describe('withPosition', () => {
  it('never moves the furthest chapter backwards', () => {
    const save = withPosition(bundle, reached(1), { chapter: 0, page: 2 })
    expect(save.position).toEqual(at('one', 2, 'h1'))
    expect(save.furthest_chapter_id).toBe('two')
  })
})

describe('spoiler gating', () => {
  it('warns only when jumping past the next unread chapter', () => {
    const save = reached(1)
    expect(jumpRevealsSpoilers(bundle, save, 0)).toBe(false)
    expect(jumpRevealsSpoilers(bundle, save, 2)).toBe(false)
    expect(jumpRevealsSpoilers(bundle, save, 3)).toBe(true)
  })

  it('unlocks everything, without warnings, once the book has been read', () => {
    const save = { ...reached(0), settings: { ...defaults, already_read: true } }
    expect(unlockedChapter(bundle, save)).toBe(3)
    expect(jumpRevealsSpoilers(bundle, save, 3)).toBe(false)
    expect(unlockedChapter(bundle, reached(1))).toBe(1)
  })
})

describe('storage round trip', () => {
  it('reads back what it wrote, and falls back to defaults for missing or broken saves', async () => {
    const storage = new MemoryStorageAdapter()
    expect(await readSave(storage, bundle, defaults)).toEqual(emptySave(defaults))

    const save = reached(1)
    await writeSave(storage, bundle, save)
    expect(await readSave(storage, bundle, defaults)).toEqual(save)

    await storage.save('tome:test:save', '{not json')
    expect(await readSave(storage, bundle, defaults)).toEqual(emptySave(defaults))
  })

  it('fills in settings added since the save was written', async () => {
    const storage = new MemoryStorageAdapter()
    const old = { version: 1, position: at('one', 0, 'h1'), furthest_chapter_id: 'one', settings: { font_scale: 1.3 } }
    await storage.save('tome:test:save', JSON.stringify(old))

    const save = await readSave(storage, bundle, defaults)
    expect(save.settings.font_scale).toBe(1.3)
    expect(save.settings.audio.music).toEqual(defaults.audio.music)
    expect(save.bookmarks).toEqual([])
  })
})
