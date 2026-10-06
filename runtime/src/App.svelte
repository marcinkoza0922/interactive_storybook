<script lang="ts">
  import { onDestroy, onMount } from 'svelte'
  import { MediaQuery } from 'svelte/reactivity'
  import { AudioDirector } from './lib/audio/director'
  import { WebAudioEngine } from './lib/audio/webaudio'
  import { loadBundle } from './lib/bundle/load'
  import type { Bundle } from './lib/bundle/types'
  import Landing from './lib/components/Landing.svelte'
  import Reader from './lib/components/Reader.svelte'
  import ThemeLayer from './lib/components/ThemeLayer.svelte'
  import { GamepadPoller } from './lib/gamepad/poller'
  import { routeGamepadAction } from './lib/gamepad/router'
  import { arriveBackward, type ReaderState } from './lib/reader/navigation'
  import { applyAudioSettings, applyDocumentSettings } from './lib/settings/apply'
  import { Progress } from './lib/state/progress.svelte'
  import { LocalStorageAdapter } from './lib/storage/adapter'
  import { defaultSettings, narrationAudible, readSave, resolvePosition, type PageRef } from './lib/storage/save'
  import { effectiveTheme } from './lib/theme/theme'

  const storage = new LocalStorageAdapter()
  const bundleUrl = new URL('book/book.json', document.baseURI).href
  /** Served by `tome preview`, which reloads the page on every rebuild. */
  const previewing = new URLSearchParams(location.search).has('preview')

  type Screen =
    | { kind: 'loading' }
    | { kind: 'error'; message: string }
    | { kind: 'landing'; bundle: Bundle; progress: Progress }
    | { kind: 'reading'; bundle: Bundle; progress: Progress; initial: ReaderState }

  let screen = $state<Screen>({ kind: 'loading' })
  let engine = $state.raw<WebAudioEngine | null>(null)
  let director: AudioDirector | null = null
  /** A line of narration is in progress or queued, heard or passing silently. */
  let narrating = $state(false)
  /** Landing music waits for the first interaction, which browsers require before audio. */
  let interacted = false
  let landingMusicPlaying = false

  const reducedMotion = new MediaQuery('(prefers-reduced-motion: reduce)')
  const assetUrl = (src: string) => new URL(src, bundleUrl).href

  /** The theme follows the reader's chapter; the title screen uses the one they'll return to. */
  const theme = $derived.by(() => {
    if (screen.kind !== 'landing' && screen.kind !== 'reading') return null
    const { bundle, progress } = screen
    const chapter = progress.save.position
      ? bundle.chapters.findIndex((c) => c.id === progress.save.position!.chapter_id)
      : -1
    return effectiveTheme(bundle, chapter === -1 ? null : chapter)
  })

  // Controllers appear to the page only after a button press, which fires gamepadconnected.
  const gamepads = new GamepadPoller(routeGamepadAction)
  onMount(() => {
    if (navigator.getGamepads?.().some((pad) => pad?.connected)) gamepads.start()
  })
  onDestroy(() => gamepads.stop())

  onMount(async () => {
    try {
      const bundle = await loadBundle(bundleUrl)
      document.title = bundle.book.title
      document.documentElement.lang = bundle.book.language
      const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)').matches
      const progress = new Progress(storage, bundle, await readSave(storage, bundle, defaultSettings(reducedMotion)))
      screen = { kind: 'landing', bundle, progress }
      // `tome preview` reloads the page on every rebuild; go straight back to where the author was.
      const position = progress.save.position
      if (previewing && position) resume(bundle, progress, position)
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

  function audioEngine(): WebAudioEngine {
    engine ??= new WebAudioEngine(bundleUrl)
    engine.unlock()
    return engine
  }

  function playLandingMusic() {
    const music = theme?.landing.music
    if (!music || landingMusicPlaying) return
    audioEngine().setMusic({ src: music, volume: 1 }, 1500)
    landingMusicPlaying = true
  }

  /** The first key or click on the title screen starts its music, unless it starts the book. */
  function onInteraction(event: Event) {
    // Audio started without a gesture (a preview reload) stays silent until the next one.
    engine?.unlock()
    if (screen.kind !== 'landing' || interacted) return
    interacted = true
    // Activating a button begins the book, whose own audio takes over.
    const onButton = (event.target as Element | null)?.closest?.('button')
    const activates = !(event instanceof KeyboardEvent) || event.key === 'Enter' || event.key === ' '
    if (!(onButton && activates)) playLandingMusic()
  }

  /** Called from the Begin/Continue click: the user gesture browsers require before audio. */
  function startReading(bundle: Bundle, progress: Progress, initial: ReaderState) {
    interacted = true
    const engine = audioEngine()
    if (landingMusicPlaying) {
      engine.setMusic(null, 1000)
      landingMusicPlaying = false
    }
    director = new AudioDirector(bundle, engine, {
      narrationEnabled: () => narrationAudible(progress.settings),
      onNarrationChange: (value) => (narrating = value),
    })
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
    startReading(bundle, progress, arriveBackward(bundle, resolvePosition(bundle, position, { keepPageOnEdit: previewing })))
  }

  function exit(bundle: Bundle, progress: Progress) {
    director?.stop()
    director = null
    narrating = false
    screen = { kind: 'landing', bundle, progress }
    playLandingMusic()
  }
</script>

<svelte:window
  onpointerdown={onInteraction}
  onkeydown={onInteraction}
  ongamepadconnected={() => gamepads.start()}
/>

{#if theme && (screen.kind === 'landing' || screen.kind === 'reading')}
  <ThemeLayer
    {theme}
    screen={screen.kind}
    motion={screen.progress.settings.special_text && !reducedMotion.current}
    {assetUrl}
  />
{/if}

{#if screen.kind === 'loading'}
  <p class="tome-message">Loading…</p>
{:else if screen.kind === 'error'}
  <p class="tome-message" role="alert">{screen.message}</p>
{:else if screen.kind === 'landing'}
  {@const { bundle, progress } = screen}
  {@const position = progress.save.position}
  <Landing
    book={bundle.book}
    theme={theme?.landing ?? {}}
    {assetUrl}
    canContinue={position !== null}
    oncontinue={() => position && resume(bundle, progress, position)}
    onbegin={() => begin(bundle, progress)}
  />
{:else}
  {@const { bundle, progress } = screen}
  <Reader
    {bundle}
    {progress}
    {narrating}
    narrationAudible={narrationAudible(progress.settings)}
    initial={screen.initial}
    {assetUrl}
    onchange={(reader, change) => director?.update(reader, change)}
    onexit={() => exit(bundle, progress)}
  />
{/if}
