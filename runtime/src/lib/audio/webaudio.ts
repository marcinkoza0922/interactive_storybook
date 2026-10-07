import type { AudioEngine, Channel, VoiceHandle } from './engine'
import type { Voice } from './state'

/** A looping track: the music, or one ambient layer. */
interface Layer {
  src: string
  gain: GainNode
  node?: AudioBufferSourceNode
}

/** Decoded audio kept for reuse; least recently used is dropped first beyond this. */
const CACHE_BUDGET_BYTES = 256 * 1024 * 1024

/**
 * Web Audio implementation. Everything plays from decoded buffers through one audio graph,
 * never from <audio> elements: browsers that block autoplay (Brave by default) refuse an
 * element's play() without a click right before it, while a resumed audio context plays
 * freely. Buffers also loop without gaps. Decoded music is large (a few minutes is tens of
 * megabytes), so the cache keeps to a budget.
 *
 * Graph: voice gain → channel gain (→ duck, for music and ambience) → master gain → speakers.
 */
export class WebAudioEngine implements AudioEngine {
  private context = new AudioContext()
  private master = this.context.createGain()
  /** Lowers music and ambience under narration, separately from the reader's volumes. */
  private duck = this.context.createGain()
  private channels: Record<Exclude<Channel, 'master'>, GainNode>
  /** Playing loops, keyed "music" or "ambient:<id>". */
  private layers = new Map<string, Layer>()
  private buffers = new Map<string, Promise<AudioBuffer | null>>()
  /** Decoded sizes, in least-recently-used order (Map keeps insertion order). */
  private sizes = new Map<string, number>()

  constructor(private baseUrl: string) {
    this.master.connect(this.context.destination)
    this.duck.connect(this.master)
    this.channels = {
      music: this.channel(this.duck),
      ambience: this.channel(this.duck),
      sfx: this.channel(),
      voice: this.channel(),
    }
  }

  /** Browsers start audio suspended; call this from a user gesture (a click or key press). */
  unlock(): Promise<void> {
    return this.context.resume()
  }

  setMusic(voice: Voice | null, fadeMs: number): void {
    this.setLayer('music', this.channels.music, voice, fadeMs)
  }

  setAmbient(id: string, voice: Voice | null, fadeMs: number): void {
    this.setLayer(`ambient:${id}`, this.channels.ambience, voice, fadeMs)
  }

  /** Fade a loop to `voice` (or silence). The same track at a new volume keeps playing. */
  private setLayer(key: string, channel: GainNode, voice: Voice | null, fadeMs: number): void {
    const playing = this.layers.get(key)
    if (playing && voice && playing.src === voice.src) {
      this.ramp(playing.gain.gain, voice.volume, fadeMs)
      return
    }

    if (playing) {
      this.layers.delete(key)
      this.ramp(playing.gain.gain, 0, fadeMs)
      this.afterFade(fadeMs, () => {
        playing.node?.stop()
        playing.gain.disconnect()
      })
    }

    if (voice) {
      const gain = this.context.createGain()
      gain.gain.value = 0
      gain.connect(channel)
      const layer: Layer = { src: voice.src, gain }
      this.layers.set(key, layer)

      this.load(voice.src).then((buffer) => {
        // Stopped or replaced while loading.
        if (!buffer || this.layers.get(key) !== layer) return
        layer.node = this.context.createBufferSource()
        layer.node.buffer = buffer
        layer.node.loop = true
        layer.node.connect(gain)
        layer.node.start()
        this.ramp(gain.gain, voice.volume, fadeMs)
      })
    }
  }

  playSfx(src: string, volume: number): void {
    this.load(src).then((buffer) => {
      if (!buffer) return
      const node = this.context.createBufferSource()
      const gain = this.context.createGain()
      node.buffer = buffer
      gain.gain.value = volume
      node.connect(gain).connect(this.channels.sfx)
      node.addEventListener('ended', () => gain.disconnect())
      node.start()
    })
  }

  /**
   * Narration plays from decoded buffers through the audio graph, not from <audio> elements:
   * once the context is unlocked, browsers that block autoplay (Brave by default) let it
   * play without a click before every line. A line started while the context is still
   * suspended waits and plays once it resumes, and its position follows the audio clock.
   */
  playVoice(src: string, volume: number, onEnded: () => void): VoiceHandle {
    const gain = this.context.createGain()
    gain.gain.value = volume
    gain.connect(this.channels.voice)

    let node: AudioBufferSourceNode | null = null
    let startedAt = 0
    let finished = false
    // A line that can't play counts as finished, so the queue (and the turn guard) moves on.
    const finish = () => {
      if (finished) return
      finished = true
      gain.disconnect()
      onEnded()
    }

    this.load(src).then((buffer) => {
      if (finished) return
      if (!buffer) return finish()
      node = this.context.createBufferSource()
      node.buffer = buffer
      node.connect(gain)
      node.addEventListener('ended', finish)
      startedAt = this.context.currentTime
      node.start()
    })

    return {
      stop: (fadeMs) => {
        if (finished) return
        finished = true
        this.ramp(gain.gain, 0, fadeMs)
        this.afterFade(fadeMs, () => {
          node?.stop()
          gain.disconnect()
        })
      },
      position: () => (node ? (this.context.currentTime - startedAt) * 1000 : 0),
    }
  }

  /** A line's length, from its decoded audio (which is then ready to play). */
  duration(src: string): Promise<number> {
    return this.load(src).then((buffer) => (buffer ? buffer.duration * 1000 : 0))
  }

  preload(srcs: string[]): void {
    for (const src of srcs) this.load(src)
  }

  setChannelVolume(channel: Channel, volume: number): void {
    const gain = channel === 'master' ? this.master : this.channels[channel]
    this.ramp(gain.gain, volume, 100)
  }

  setDucking(level: number, fadeMs: number): void {
    this.ramp(this.duck.gain, level, fadeMs)
  }

  private channel(output: AudioNode = this.master): GainNode {
    const gain = this.context.createGain()
    gain.connect(output)
    return gain
  }

  private load(src: string): Promise<AudioBuffer | null> {
    let buffer = this.buffers.get(src)
    if (buffer) this.touch(src)
    if (!buffer) {
      buffer = fetch(this.url(src))
        .then((response) => {
          if (!response.ok) throw new Error(`${response.status} ${response.statusText}`)
          return response.arrayBuffer()
        })
        .then((data) => this.context.decodeAudioData(data))
        .then((decoded) => {
          this.sizes.set(src, decoded.length * decoded.numberOfChannels * 4)
          this.evict()
          return decoded
        })
        .catch((error) => {
          console.warn(`Could not load sound: ${src}`, error)
          return null
        })
      this.buffers.set(src, buffer)
    }
    return buffer
  }

  private touch(src: string): void {
    const size = this.sizes.get(src)
    if (size === undefined) return
    this.sizes.delete(src)
    this.sizes.set(src, size)
  }

  /** Drop least recently used decoded audio over the budget, never a loop that's playing. */
  private evict(): void {
    const playing = new Set([...this.layers.values()].map((l) => l.src))
    let total = [...this.sizes.values()].reduce((a, b) => a + b, 0)
    for (const [src, size] of this.sizes) {
      if (total <= CACHE_BUDGET_BYTES) break
      if (playing.has(src)) continue
      this.sizes.delete(src)
      this.buffers.delete(src)
      total -= size
    }
  }

  /** Move a gain smoothly from wherever it is now, cancelling any ramp in progress. */
  private ramp(param: AudioParam, value: number, ms: number): void {
    const now = this.context.currentTime
    param.cancelScheduledValues(now)
    param.setValueAtTime(param.value, now)
    if (ms <= 0) param.setValueAtTime(value, now)
    else param.linearRampToValueAtTime(value, now + ms / 1000)
  }

  private afterFade(ms: number, cleanup: () => void): void {
    setTimeout(cleanup, ms + 50)
  }

  private url(src: string): string {
    return new URL(src, this.baseUrl).href
  }
}
