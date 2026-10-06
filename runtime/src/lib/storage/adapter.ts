/**
 * All persistence goes through this interface so that other backends (an Electron file,
 * a sync server, Steam Cloud) can replace browser storage without touching the reader.
 * Async so that remote backends fit the same shape.
 */
export interface StorageAdapter {
  load(key: string): Promise<string | null>
  save(key: string, value: string): Promise<void>
  remove(key: string): Promise<void>
}

/** Browser localStorage. Fails soft: private windows and blocked storage just lose progress. */
export class LocalStorageAdapter implements StorageAdapter {
  async load(key: string): Promise<string | null> {
    try {
      return localStorage.getItem(key)
    } catch {
      return null
    }
  }

  async save(key: string, value: string): Promise<void> {
    try {
      localStorage.setItem(key, value)
    } catch (error) {
      console.warn('Could not save reading progress:', error)
    }
  }

  async remove(key: string): Promise<void> {
    try {
      localStorage.removeItem(key)
    } catch {
      // Nothing to remove if storage is unavailable.
    }
  }
}

export class MemoryStorageAdapter implements StorageAdapter {
  private values = new Map<string, string>()

  async load(key: string) {
    return this.values.get(key) ?? null
  }

  async save(key: string, value: string) {
    this.values.set(key, value)
  }

  async remove(key: string) {
    this.values.delete(key)
  }
}
