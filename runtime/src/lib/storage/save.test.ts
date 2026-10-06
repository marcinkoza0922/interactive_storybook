import { describe, expect, it } from 'vitest'
import type { Bundle } from '../bundle/types'
import { MemoryStorageAdapter } from './adapter'
import { nextSave, readSave, resolvePosition, writeSave, type SaveState } from './save'

const pages = (n: number) => Array.from({ length: n }, (_, i) => ({ blocks: [{ id: `b${i}`, html: '' }] }))

const bundle: Bundle = {
  bundle_schema_version: 1,
  book: { id: 'test', title: 'Test', author: 'A', language: 'en' },
  chapters: [
    { id: 'one', title: 'One', content_hash: 'h1', pages: pages(3) },
    { id: 'two', title: 'Two', content_hash: 'h2', pages: pages(2) },
  ],
}

const saved = (chapter_id: string, page: number, content_hash: string): SaveState => ({
  version: 1,
  position: { chapter_id, page, content_hash },
  furthest_chapter_id: chapter_id,
})

describe('resolvePosition', () => {
  it('restores an unchanged chapter exactly', () => {
    expect(resolvePosition(bundle, saved('two', 1, 'h2'))).toEqual({ chapter: 1, page: 1 })
  })

  it('resets to the chapter start when the chapter content changed', () => {
    expect(resolvePosition(bundle, saved('two', 1, 'stale'))).toEqual({ chapter: 1, page: 0 })
  })

  it('falls back to the book start when the chapter no longer exists', () => {
    expect(resolvePosition(bundle, saved('gone', 1, 'h2'))).toEqual({ chapter: 0, page: 0 })
  })
})

describe('nextSave', () => {
  it('never moves the furthest chapter backwards', () => {
    const save = nextSave(bundle, { chapter: 0, page: 2 }, saved('two', 0, 'h2'))
    expect(save.position).toEqual({ chapter_id: 'one', page: 2, content_hash: 'h1' })
    expect(save.furthest_chapter_id).toBe('two')
  })
})

describe('storage round trip', () => {
  it('reads back what it wrote and ignores garbage', async () => {
    const storage = new MemoryStorageAdapter()
    expect(await readSave(storage, bundle)).toBeNull()

    const save = nextSave(bundle, { chapter: 1, page: 0 }, null)
    await writeSave(storage, bundle, save)
    expect(await readSave(storage, bundle)).toEqual(save)

    await storage.save('tome:test:save', '{not json')
    expect(await readSave(storage, bundle)).toBeNull()
  })
})
