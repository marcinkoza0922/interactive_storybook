import type { Voice } from './state'

export type Channel = 'music' | 'ambience' | 'sfx'

/**
 * Plays sound. Deliberately dumb: the director decides what should be heard and when;
 * the engine only performs transitions. Calls never throw; failures are logged and skipped.
 */
export interface AudioEngine {
  /** Fade to `voice` (or silence). Changing the volume of the playing track keeps it playing. */
  setMusic(voice: Voice | null, fadeMs: number): void
  /** Start, change or stop (null) one ambient layer. */
  setAmbient(id: string, voice: Voice | null, fadeMs: number): void
  playSfx(src: string, volume: number): void
  /** Fetch and decode in advance so cues fire on time. */
  preload(srcs: string[]): void
  setChannelVolume(channel: Channel, volume: number): void
}
