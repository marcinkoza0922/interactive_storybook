<script lang="ts">
  import { onMount } from 'svelte'
  import { loadBundle } from './lib/bundle/load'
  import type { Bundle } from './lib/bundle/types'
  import Landing from './lib/components/Landing.svelte'
  import Reader from './lib/components/Reader.svelte'
  import { arriveBackward, type Position, type ReaderState } from './lib/reader/navigation'
  import { LocalStorageAdapter } from './lib/storage/adapter'
  import { nextSave, readSave, resolvePosition, writeSave, type SaveState } from './lib/storage/save'

  const storage = new LocalStorageAdapter()

  type Screen =
    | { kind: 'loading' }
    | { kind: 'error'; message: string }
    | { kind: 'landing'; bundle: Bundle }
    | { kind: 'reading'; bundle: Bundle; initial: ReaderState }

  let screen = $state<Screen>({ kind: 'loading' })
  let save = $state<SaveState | null>(null)

  onMount(async () => {
    try {
      const bundle = await loadBundle(`${import.meta.env.BASE_URL}book/book.json`)
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

  function begin(bundle: Bundle) {
    // Starting over keeps the furthest chapter reached, so unlocked references stay unlocked.
    savePosition(bundle, { chapter: 0, page: 0 })
    screen = { kind: 'reading', bundle, initial: { position: { chapter: 0, page: 0 }, revealed: 0, direction: 'forward' } }
  }

  function resume(bundle: Bundle, save: SaveState) {
    // Resuming lands on an already-seen page, so it is shown fully revealed.
    screen = { kind: 'reading', bundle, initial: arriveBackward(bundle, resolvePosition(bundle, save)) }
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
    onpositionchange={(position) => savePosition(bundle, position)}
    onexit={() => (screen = { kind: 'landing', bundle })}
  />
{/if}
