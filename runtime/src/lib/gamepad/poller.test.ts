import { beforeEach, describe, expect, it } from 'vitest'
import { GamepadPoller, REPEAT_DELAY_MS, REPEAT_INTERVAL_MS, type GamepadAction } from './poller'

interface FakePad {
  index: number
  connected: boolean
  buttons: { pressed: boolean }[]
  axes: number[]
}

const pad = (index = 0): FakePad => ({
  index,
  connected: true,
  buttons: Array.from({ length: 17 }, () => ({ pressed: false })),
  axes: [0, 0, 0, 0],
})

describe('GamepadPoller', () => {
  let pads: FakePad[]
  let time: number
  let actions: GamepadAction[]
  let poller: GamepadPoller

  const at = (ms: number) => {
    time = ms
    poller.poll()
  }

  beforeEach(() => {
    pads = [pad()]
    time = 0
    actions = []
    poller = new GamepadPoller((a) => actions.push(a), {
      getGamepads: () => pads as unknown as Gamepad[],
      now: () => time,
      requestFrame: () => 1,
      cancelFrame: () => {},
    })
  })

  it('fires a button once per press, not while held', () => {
    pads[0].buttons[0].pressed = true
    at(0)
    at(16)
    at(1000)
    pads[0].buttons[0].pressed = false
    at(1016)
    pads[0].buttons[0].pressed = true
    at(1032)
    expect(actions).toEqual(['confirm', 'confirm'])
  })

  it('maps the standard layout', () => {
    for (const index of [1, 2, 3, 4, 5, 9]) pads[0].buttons[index].pressed = true
    at(0)
    expect(actions.sort()).toEqual(['cancel', 'illustration', 'menu', 'next', 'prev', 'references'])
  })

  it('repeats held directions after a delay', () => {
    pads[0].buttons[13].pressed = true
    for (let t = 0; t <= REPEAT_DELAY_MS + 2 * REPEAT_INTERVAL_MS; t += 10) at(t)
    expect(actions).toEqual(['down', 'down', 'down', 'down'])
  })

  it('treats the left stick as a D-pad, with hysteresis', () => {
    pads[0].axes[0] = 0.5
    at(0)
    expect(actions).toEqual([]) // below the press threshold
    pads[0].axes[0] = 0.7
    at(16)
    pads[0].axes[0] = 0.45 // still above the release threshold: held, no new press
    at(32)
    pads[0].axes[0] = 0.2
    at(48)
    pads[0].axes[1] = -0.9
    at(64)
    expect(actions).toEqual(['right', 'up'])
  })

  it('reports whether a controller is still connected', () => {
    expect(poller.poll()).toBe(true)
    pads[0].connected = false
    expect(poller.poll()).toBe(false)
  })
})
