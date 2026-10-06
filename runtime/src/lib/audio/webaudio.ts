import type { AudioEngine, Channel } from './engine'
import type { Voice } from './state'

interface MusicVoice {
  src: string
  element: HTMLAudioElement
  source: MediaElementAudioSourceNode
  gain: GainNode
}

interface AmbientVoice {
  src: string
  gain: GainNode
  node?: AudioBufferSourceNode
}

/**
 * Web Audio implementation. Music streams through an <audio> element, since a long track
 * decoded into memory costs tens of megabytes. Ambient loops and sound effects are short,
 * so they are decoded buffers, which loop without gaps and start without latency.
 *
 * Graph: voice gain → channel gain → master gain → speakers.
 */
export class WebAudioEngine implements AudioEngine {
  private context = new AudioContext()
  private master = this.context.createGain()
  private channels: Record<Channel, GainNode>
  private music: MusicVoice | null = null
  private ambient = new Map<string, AmbientVoice>()
  private buffers = new Map<string, Promise<AudioBuffer | null>>()

  constructor(private baseUrl: string) {
    this.master.connect(this.context.destination)
    this.channels = { music: this.channel(), ambience: this.channel(), sfx: this.channel() }
  }

  /** Browsers start audio suspended; call this from a user gesture (a click or key press). */
  unlock(): Promise<void> {
    return this.context.resume()
  }

  setMusic(voice: Voice | null, fadeMs: number): void {
    const playing = this.music
    if (playing && voice && playing.src === voice.src) {
      this.ramp(playing.gain.gain, voice.volume, fadeMs)
      return
    }

    if (playing) {
      this.music = null
      this.ramp(playing.gain.gain, 0, fadeMs)
      this.afterFade(fadeMs, () => {
        playing.element.pause()
        playing.element.removeAttribute('src')
        playing.source.disconnect()
        playing.gain.disconnect()
      })
    }

    if (voice) {
      const element = new Audio(this.url(voice.src))
      element.loop = true
      element.addEventListener('error', () => console.warn(`Could not load music: ${voice.src}`))

      const source = this.context.createMediaElementSource(element)
      const gain = this.context.createGain()
      gain.gain.value = 0
      source.connect(gain).connect(this.channels.music)

      this.music = { src: voice.src, element, source, gain }
      element.play().catch((error) => console.warn(`Could not play music: ${voice.src}`, error))
      this.ramp(gain.gain, voice.volume, fadeMs)
    }
  }

  setAmbient(id: string, voice: Voice | null, fadeMs: number): void {
    const playing = this.ambient.get(id)
    if (playing && voice && playing.src === voice.src) {
      this.ramp(playing.gain.gain, voice.volume, fadeMs)
      return
    }

    if (playing) {
      this.ambient.delete(id)
      this.ramp(playing.gain.gain, 0, fadeMs)
      this.afterFade(fadeMs, () => {
        playing.node?.stop()
        playing.gain.disconnect()
      })
    }

    if (voice) {
      const gain = this.context.createGain()
      gain.gain.value = 0
      gain.connect(this.channels.ambience)
      const layer: AmbientVoice = { src: voice.src, gain }
      this.ambient.set(id, layer)

      this.load(voice.src).then((buffer) => {
        // Stopped or replaced while loading.
        if (!buffer || this.ambient.get(id) !== layer) return
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

  preload(srcs: string[]): void {
    for (const src of srcs) this.load(src)
  }

  setChannelVolume(channel: Channel, volume: number): void {
    this.ramp(this.channels[channel].gain, volume, 100)
  }

  private channel(): GainNode {
    const gain = this.context.createGain()
    gain.connect(this.master)
    return gain
  }

  private load(src: string): Promise<AudioBuffer | null> {
    let buffer = this.buffers.get(src)
    if (!buffer) {
      buffer = fetch(this.url(src))
        .then((response) => {
          if (!response.ok) throw new Error(`${response.status} ${response.statusText}`)
          return response.arrayBuffer()
        })
        .then((data) => this.context.decodeAudioData(data))
        .catch((error) => {
          console.warn(`Could not load sound: ${src}`, error)
          return null
        })
      this.buffers.set(src, buffer)
    }
    return buffer
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
