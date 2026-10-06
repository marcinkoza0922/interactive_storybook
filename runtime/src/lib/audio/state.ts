import type { Bundle, Cue } from '../bundle/types'

export interface Voice {
  src: string
  volume: number
}

/** What should be sounding continuously. One-shot sound effects aren't part of it. */
export interface AudioState {
  music: Voice | null
  ambient: Readonly<Record<string, Voice>>
}

export const SILENCE: AudioState = { music: null, ambient: {} }

export function applyCue(state: AudioState, cue: Cue): AudioState {
  switch (cue.kind) {
    case 'music':
      return { ...state, music: { src: cue.src, volume: cue.volume ?? 1 } }
    case 'music_stop':
      return { ...state, music: null }
    case 'ambient':
      return { ...state, ambient: { ...state.ambient, [cue.id]: { src: cue.src, volume: cue.volume ?? 1 } } }
    case 'ambient_stop': {
      const { [cue.id]: _, ...rest } = state.ambient
      return { ...state, ambient: rest }
    }
    case 'sfx':
      return state
  }
}

/**
 * The audio state at the end of every page, indexed [chapter][page]. Music and ambience
 * carry across pages and chapters until a cue changes them, so this folds over the whole book.
 */
export function pageEndStates(bundle: Bundle): AudioState[][] {
  let state = SILENCE
  return bundle.chapters.map((chapter) =>
    chapter.pages.map((page) => {
      for (const block of page.blocks) for (const cue of block.cues ?? []) state = applyCue(state, cue)
      return state
    }),
  )
}

export function sameVoice(a: Voice | null | undefined, b: Voice | null | undefined): boolean {
  return a?.src === b?.src && a?.volume === b?.volume
}
