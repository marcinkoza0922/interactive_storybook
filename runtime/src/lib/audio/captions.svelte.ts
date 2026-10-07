export interface Caption {
  id: number
  text: string
}

/**
 * What the caption strip shows: captions of sounds that just started, each for a while, and
 * the soundscape, the captions of the music and ambience still sounding.
 */
export class Captions {
  /** Oldest first. */
  current = $state<Caption[]>([])
  soundscape = $state<string[]>([])
  private nextId = 0
  private timers = new Map<number, ReturnType<typeof setTimeout>>()

  /** Show `text` for `durationMs`. A caption already showing is extended, not repeated. */
  show(text: string, durationMs: number): void {
    const existing = this.current.find((c) => c.text === text)
    const id = existing?.id ?? this.nextId++
    if (!existing) this.current.push({ id, text })
    clearTimeout(this.timers.get(id))
    this.timers.set(
      id,
      setTimeout(() => {
        this.timers.delete(id)
        this.current = this.current.filter((c) => c.id !== id)
      }, durationMs),
    )
  }

  /** Forget everything, e.g. when leaving the reader. */
  clear(): void {
    for (const timer of this.timers.values()) clearTimeout(timer)
    this.timers.clear()
    this.current = []
    this.soundscape = []
  }
}
