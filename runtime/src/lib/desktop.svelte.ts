/** What the desktop app's preload script exposes. Absent on the web. */
interface DesktopBridge {
  quit(): void
  isFullscreen(): Promise<boolean>
  setFullscreen(on: boolean): void
  onFullscreenChange(callback: (on: boolean) => void): () => void
}

/** Desktop-only controls: quitting and full screen. */
class Desktop {
  /** Kept in step with the window, however full screen changes (the setting, F11, …). */
  fullscreen = $state(false)

  constructor(private bridge: DesktopBridge) {
    bridge.isFullscreen().then((on) => (this.fullscreen = on))
    bridge.onFullscreenChange((on) => (this.fullscreen = on))
  }

  setFullscreen(on: boolean): void {
    this.bridge.setFullscreen(on)
  }

  quit(): void {
    this.bridge.quit()
  }
}

const bridge = (window as { tomeDesktop?: DesktopBridge }).tomeDesktop

/** Null when running in a browser. */
export const desktop: Desktop | null = bridge ? new Desktop(bridge) : null
