<script lang="ts">
  import { onMount } from 'svelte'
  import { AudioDirector } from './lib/audio/director'
  import { WebAudioEngine } from './lib/audio/webaudio'
  import { loadBundle } from './lib/bundle/load'
  import type { Bundle } from './lib/bundle/types'
  import Landing from './lib/components/Landing.svelte'
  import Reader from './lib/components/Reader.svelte'
  import { arriveBackward, type ReaderState } from './lib/reader/navigation'
  import { applyAudioSettings, applyDocumentSettings } from './lib/settings/apply'
  import { Progress } from './lib/state/progress.svelte'
  import { LocalStorageAdapter } from './lib/storage/adapter'
  import { defaultSettings, readSave, resolvePosition, type PageRef } from './lib/storage/save'

  const storage = new LocalStorageAdapter()
  const bundleUrl = new URL('book/book.json', document.baseURI).href

  type Screen =
    | { kind: 'loading' }
    | { kind: 'error'; message: string }
    | { kind: 'landing'; bundle: Bundle; progress: Progress }
    | { kind: 'reading'; bundle: Bundle; progress: Progress; initial: ReaderState }

  let screen = $state<Screen>({ kind: 'loading' })
  let engine = $state.raw<WebAudioEngine | null>(null)
  let director: AudioDirector | null = null

  onMount(async () => {
    try {
      const bundle = await loadBundle(bundleUrl)
      document.title = bundle.book.title
      document.documentElement.lang = bundle.book.language
      const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)').matches
      const progress = new Progress(storage, bundle, await readSave(storage, bundle, defaultSettings(reducedMotion)))
      screen = { kind: 'landing', bundle, progress }
    } catch (error) {
      screen = { kind: 'error', message: error instanceof Error ? error.message : String(error) }
    }
  })

  // Reader settings take effect immediately, wherever they're changed.
  $effect(() => {
    if (screen.kind === 'landing' || screen.kind === 'reading') {
      applyDocumentSettings(document.documentElement, screen.progress.settings)
      if (engine) applyAudioSettings(engine, screen.progress.settings)
    }
  })

  /** Called from the Begin/Continue click: the user gesture browsers require before audio. */
  function startReading(bundle: Bundle, progress: Progress, initial: ReaderState) {
    engine ??= new WebAudioEngine(bundleUrl)
    engine.unlock()
    director = new AudioDirector(bundle, engine)
    director.update(initial, 'enter')
    screen = { kind: 'reading', bundle, progress, initial }
  }

  function begin(bundle: Bundle, progress: Progress) {
    // Starting over keeps the furthest chapter reached, so unlocked references stay unlocked.
    progress.setPosition({ chapter: 0, page: 0 })
    startReading(bundle, progress, { position: { chapter: 0, page: 0 }, revealed: 0, direction: 'forward' })
  }

  function resume(bundle: Bundle, progress: Progress, position: PageRef) {
    // Resuming lands on an already-seen page: shown fully revealed, its audio restored at once.
    startReading(bundle, progress, arriveBackward(bundle, resolvePosition(bundle, position)))
  }

  function exit(bundle: Bundle, progress: Progress) {
    director?.stop()
    director = null
    screen = { kind: 'landing', bundle, progress }
  }
</script>

{#if screen.kind === 'loading'}
  <p class="tome-message">Loading…</p>
{:else if screen.kind === 'error'}
  <p class="tome-message" role="alert">{screen.message}</p>
{:else if screen.kind === 'landing'}
  {@const { bundle, progress } = screen}
  {@const position = progress.save.position}
  <Landing
    book={bundle.book}
    canContinue={position !== null}
    oncontinue={() => position && resume(bundle, progress, position)}
    onbegin={() => begin(bundle, progress)}
  />
{:else}
  {@const { bundle, progress } = screen}
  <Reader
    {bundle}
    {progress}
    initial={screen.initial}
    assetUrl={(src) => new URL(src, bundleUrl).href}
    onchange={(reader, change) => director?.update(reader, change)}
    onexit={() => exit(bundle, progress)}
  />
{/if}
