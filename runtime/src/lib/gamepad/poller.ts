export type GamepadAction =
  | 'confirm'
  | 'cancel'
  | 'menu'
  | 'references'
  | 'illustration'
  | 'prev'
  | 'next'
  | 'up'
  | 'down'
  | 'left'
  | 'right'

/** Button indices in the W3C "standard" gamepad layout (Xbox naming; Steam Input maps to it). */
const BUTTONS: [index: number, action: GamepadAction][] = [
  [0, 'confirm'], // A
  [1, 'cancel'], // B
  [2, 'illustration'], // X
  [3, 'references'], // Y
  [4, 'prev'], // LB
  [5, 'next'], // RB
  [8, 'menu'], // Back / View
  [9, 'menu'], // Start / Menu
  [12, 'up'],
  [13, 'down'],
  [14, 'left'],
  [15, 'right'],
]

/** Directions repeat while held, like a key; everything else fires once per press. */
const REPEATING = new Set<GamepadAction>(['up', 'down', 'left', 'right'])
export const REPEAT_DELAY_MS = 400
export const REPEAT_INTERVAL_MS = 120

// The left stick acts as a D-pad. Separate press and release thresholds stop a stick
// resting near the edge from flickering on and off.
const STICK_PRESS = 0.6
const STICK_RELEASE = 0.35

export interface PollerDeps {
  getGamepads: () => readonly (Gamepad | null)[]
  now: () => number
  requestFrame: (callback: () => void) => number
  cancelFrame: (handle: number) => void
}

const browserDeps: PollerDeps = {
  getGamepads: () => navigator.getGamepads?.() ?? [],
  now: () => performance.now(),
  requestFrame: (callback) => requestAnimationFrame(callback),
  cancelFrame: (handle) => cancelAnimationFrame(handle),
}

/**
 * Turns controller state into actions. The Gamepad API has no input events, so this polls
 * once per animation frame while a controller is connected.
 */
export class GamepadPoller {
  /** Held inputs, keyed by `<pad>:<action>`, with when they started and last fired. */
  private held = new Map<string, { since: number; last: number }>()
  private frame: number | null = null

  constructor(
    private onAction: (action: GamepadAction) => void,
    private deps: PollerDeps = browserDeps,
  ) {}

  start(): void {
    if (this.frame !== null) return
    const loop = () => {
      this.frame = null
      if (this.poll()) this.frame = this.deps.requestFrame(loop)
    }
    this.frame = this.deps.requestFrame(loop)
  }

  stop(): void {
    if (this.frame !== null) this.deps.cancelFrame(this.frame)
    this.frame = null
    this.held.clear()
  }

  /** Read every controller once. Returns whether any is still connected. */
  poll(): boolean {
    const now = this.deps.now()
    const active = new Set<string>()
    let connected = false

    for (const pad of this.deps.getGamepads()) {
      if (!pad?.connected) continue
      connected = true
      for (const action of this.pressedActions(pad)) active.add(`${pad.index}:${action}`)
    }

    for (const key of active) {
      const action = key.slice(key.indexOf(':') + 1) as GamepadAction
      const held = this.held.get(key)
      if (!held) {
        this.held.set(key, { since: now, last: now })
        this.onAction(action)
      } else if (REPEATING.has(action) && now - held.since >= REPEAT_DELAY_MS && now - held.last >= REPEAT_INTERVAL_MS) {
        held.last = now
        this.onAction(action)
      }
    }
    for (const key of this.held.keys()) if (!active.has(key)) this.held.delete(key)

    return connected
  }

  private pressedActions(pad: Gamepad): Set<GamepadAction> {
    const actions = new Set<GamepadAction>()
    for (const [index, action] of BUTTONS) if (pad.buttons[index]?.pressed) actions.add(action)

    const [x = 0, y = 0] = pad.axes
    const stick = (action: GamepadAction, value: number) => {
      const threshold = this.held.has(`${pad.index}:${action}`) ? STICK_RELEASE : STICK_PRESS
      if (value >= threshold) actions.add(action)
    }
    stick('left', -x)
    stick('right', x)
    stick('up', -y)
    stick('down', y)
    return actions
  }
}
