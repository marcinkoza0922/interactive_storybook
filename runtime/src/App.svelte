<script lang="ts">
  import { onMount } from 'svelte'
  import { AudioDirector } from './lib/audio/director'
  import { WebAudioEngine } from './lib/audio/webaudio'
  import { loadBundle } from './lib/bundle/load'
  import type { Bundle } from './lib/bundle/types'
  import Landing from './lib/components/Landing.svelte'
  import Reader from './lib/components/Reader.svelte'
  import { arriveBackward, type Position, type ReaderState } from './lib/reader/navigation'
  import { LocalStorageAdapter } from './lib/storage/adapter'
  import { nextSave, readSave, resolvePosition, writeSave, type SaveState } from './lib/storage/save'

  const storage = new LocalStorageAdapter()
  const bundleUrl = new URL('book/book.json', document.baseURI).href

  type Screen =
    | { kind: 'loading' }
    | { kind: 'error'; message: string }
    | { kind: 'landing'; bundle: Bundle }
    | { kind: 'reading'; bundle: Bundle; initial: ReaderState }

  let screen = $state<Screen>({ kind: 'loading' })
  let save = $state<SaveState | null>(null)
  let engine: WebAudioEngine | null = null
  let director: AudioDirector | null = null

  onMount(async () => {
    try {
      const bundle = await loadBundle(bundleUrl)
      document.title = bundle.book.title
      document.documentElement.lang = bundle.book.language
      save = await readSave(storage, bundle)
      screen = { kind: 'landing', bundle }
    } catch (error) {
      screen = { kind: 'error', message: error instanceof Error ? error.message : String(error) }
    }
  })

  function savePosition(bundle: Bundle, position: Position) {
    save = nextSave(bundle, position, save)
    writeSave(storage, bundle, save)
  }

  /** Called from the Begin/Continue click: the user gesture browsers require before audio. */
  function startReading(bundle: Bundle, initial: ReaderState) {
    engine ??= new WebAudioEngine(bundleUrl)
    engine.unlock()
    director = new AudioDirector(bundle, engine)
    director.update(initial, 'enter')
    screen = { kind: 'reading', bundle, initial }
  }

  function begin(bundle: Bundle) {
    // Starting over keeps the furthest chapter reached, so unlocked references stay unlocked.
    savePosition(bundle, { chapter: 0, page: 0 })
    startReading(bundle, { position: { chapter: 0, page: 0 }, revealed: 0, direction: 'forward' })
  }

  function resume(bundle: Bundle, save: SaveState) {
    // Resuming lands on an already-seen page: shown fully revealed, its audio restored at once.
    startReading(bundle, arriveBackward(bundle, resolvePosition(bundle, save)))
  }

  function exit(bundle: Bundle) {
    director?.stop()
    director = null
    screen = { kind: 'landing', bundle }
  }
</script>

{#if screen.kind === 'loading'}
  <p class="tome-message">Loading…</p>
{:else if screen.kind === 'error'}
  <p class="tome-message" role="alert">{screen.message}</p>
{:else if screen.kind === 'landing'}
  {@const bundle = screen.bundle}
  <Landing
    book={bundle.book}
    canContinue={save !== null}
    oncontinue={() => save && resume(bundle, save)}
    onbegin={() => begin(bundle)}
  />
{:else}
  {@const bundle = screen.bundle}
  <Reader
    {bundle}
    initial={screen.initial}
    furthestChapter={Math.max(0, bundle.chapters.findIndex((c) => c.id === save?.furthest_chapter_id))}
    assetUrl={(src) => new URL(src, bundleUrl).href}
    onpositionchange={(position) => savePosition(bundle, position)}
    onchange={(reader, change) => director?.update(reader, change)}
    onexit={() => exit(bundle)}
  />
{/if}
