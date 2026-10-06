import type { Bundle } from '../bundle/types'
import type { Position } from '../reader/navigation'
import type { StorageAdapter } from '../storage/adapter'
import {
  pageRef,
  unlockedChapter,
  withPosition,
  writeSave,
  type Bookmark,
  type Highlight,
  type SaveState,
  type Settings,
} from '../storage/save'

// randomUUID needs a secure context, which a web build on plain http isn't.
const newId = () => crypto.randomUUID?.() ?? `${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`

/** The reader's saved state, reactive for the UI and persisted on every change. */
export class Progress {
  save: SaveState = $state()!

  constructor(
    private storage: StorageAdapter,
    private bundle: Bundle,
    initial: SaveState,
  ) {
    this.save = initial
  }

  get settings(): Settings {
    return this.save.settings
  }

  /** Index of the chapter up to which references are unlocked. */
  get unlockedChapter(): number {
    return unlockedChapter(this.bundle, this.save)
  }

  setPosition(position: Position): void {
    this.save = withPosition(this.bundle, this.save, position)
    this.persist()
  }

  /**
   * Start the book over: forget the position, the furthest chapter (relocking references) and
   * the already-read flag. Bookmarks and highlights go too only if asked; settings stay.
   */
  resetProgress({ annotations }: { annotations: boolean }): void {
    this.save.position = null
    this.save.furthest_chapter_id = null
    this.save.settings.already_read = false
    if (annotations) {
      this.save.bookmarks = []
      this.save.highlights = []
    }
    this.persist()
  }

  updateSettings(changes: Partial<Settings>): void {
    Object.assign(this.save.settings, changes)
    this.persist()
  }

  bookmarkAt(position: Position): Bookmark | undefined {
    const ref = pageRef(this.bundle, position)
    return this.save.bookmarks.find((b) => b.chapter_id === ref.chapter_id && b.page === ref.page)
  }

  addBookmark(position: Position, label: string): void {
    this.save.bookmarks.push({ id: newId(), ...pageRef(this.bundle, position), label, created_at: new Date().toISOString() })
    this.persist()
  }

  /** Bookmark the page, or remove its bookmark if it has one. */
  toggleBookmark(position: Position): void {
    const existing = this.bookmarkAt(position)
    if (existing) this.removeBookmark(existing.id)
    else this.addBookmark(position, `${this.bundle.chapters[position.chapter].title} · page ${position.page + 1}`)
  }

  renameBookmark(id: string, label: string): void {
    const bookmark = this.save.bookmarks.find((b) => b.id === id)
    if (bookmark) bookmark.label = label
    this.persist()
  }

  removeBookmark(id: string): void {
    this.save.bookmarks = this.save.bookmarks.filter((b) => b.id !== id)
    this.persist()
  }

  addHighlight(position: Position, highlight: Pick<Highlight, 'start' | 'end' | 'text'>): void {
    this.save.highlights.push({
      id: newId(),
      ...pageRef(this.bundle, position),
      ...highlight,
      note: '',
      created_at: new Date().toISOString(),
    })
    this.persist()
  }

  setHighlightNote(id: string, note: string): void {
    const highlight = this.save.highlights.find((h) => h.id === id)
    if (highlight) highlight.note = note
    this.persist()
  }

  removeHighlight(id: string): void {
    this.save.highlights = this.save.highlights.filter((h) => h.id !== id)
    this.persist()
  }

  private persist(): void {
    writeSave(this.storage, this.bundle, $state.snapshot(this.save) as SaveState)
  }
}
