import { describe, expect, it } from 'vitest'
import type { Bundle } from '../bundle/types'
import { MemoryStorageAdapter } from '../storage/adapter'
import { defaultSettings, emptySave, readSave } from '../storage/save'
import { Progress } from './progress.svelte'

const pages = (n: number) => Array.from({ length: n }, (_, i) => ({ blocks: [{ id: `b${i}`, html: '' }] }))
const bundle: Bundle = {
  bundle_schema_version: 1,
  book: { id: 'test', title: 'Test', author: 'A', language: 'en' },
  chapters: [
    { id: 'one', title: 'One', content_hash: 'h1', pages: pages(2) },
    { id: 'two', title: 'Two', content_hash: 'h2', pages: pages(2) },
  ],
}

function readBook() {
  const storage = new MemoryStorageAdapter()
  const progress = new Progress(storage, bundle, emptySave(defaultSettings(false)))
  progress.setPosition({ chapter: 1, page: 1 })
  progress.addBookmark({ chapter: 1, page: 0 }, 'Here')
  progress.addHighlight({ chapter: 0, page: 0 }, { start: { block_id: 'b0', offset: 0 }, end: { block_id: 'b0', offset: 1 }, text: 'x' })
  progress.updateSettings({ already_read: true, font_scale: 1.3 })
  return { storage, progress }
}

describe('resetting progress', () => {
  it('forgets the position, unlocks and already-read flag, but keeps settings and notes by default', async () => {
    const { storage, progress } = readBook()
    progress.resetProgress({ annotations: false })

    const saved = await readSave(storage, bundle, defaultSettings(false))
    expect(saved.position).toBeNull()
    expect(saved.furthest_chapter_id).toBeNull()
    expect(saved.settings.already_read).toBe(false)
    expect(saved.settings.font_scale).toBe(1.3)
    expect(saved.bookmarks).toHaveLength(1)
    expect(saved.highlights).toHaveLength(1)
    expect(progress.unlockedChapter).toBe(0)
  })

  it('deletes bookmarks and highlights when asked', async () => {
    const { storage, progress } = readBook()
    progress.resetProgress({ annotations: true })
    const saved = await readSave(storage, bundle, defaultSettings(false))
    expect([saved.bookmarks.length, saved.highlights.length]).toEqual([0, 0])
  })
})
