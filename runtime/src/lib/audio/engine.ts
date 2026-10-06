import type { Voice } from './state'

export type Channel = 'master' | 'music' | 'ambience' | 'sfx' | 'voice'

/** A playing line of narration. */
export interface VoiceHandle {
  /** Stop early, fading out; `onEnded` is not called. */
  stop(fadeMs: number): void
}

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
  /** Play a line of narration; `onEnded` fires when it finishes or fails to play. */
  playVoice(src: string, volume: number, onEnded: () => void): VoiceHandle
  /** How long a line of narration lasts, in milliseconds (0 if unknown), without playing it. */
  voiceDuration(src: string): Promise<number>
  /** Fetch and decode in advance so cues fire on time. */
  preload(srcs: string[]): void
  setChannelVolume(channel: Channel, volume: number): void
  /** Scale music and ambience (1 = full), on top of the reader's own volumes, e.g. under narration. */
  setDucking(level: number, fadeMs: number): void
}
