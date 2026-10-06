import type { GamepadAction } from './poller'

const FOCUSABLE = 'button:not([disabled]), input:not([disabled]), textarea:not([disabled]), select:not([disabled]), a[href]'

function focusables(container: Element): HTMLElement[] {
  return [...container.querySelectorAll<HTMLElement>(FOCUSABLE)].filter((el) => el.getClientRects().length > 0)
}

function focus(element: HTMLElement): void {
  // focusVisible: show the focus ring, which browsers otherwise reserve for keyboard use.
  element.focus({ focusVisible: true } as FocusOptions)
  element.scrollIntoView({ block: 'nearest' })
}

/** Move focus to the next or previous control in `container`, stopping at the ends. */
export function moveFocus(container: Element, delta: 1 | -1): void {
  const controls = focusables(container)
  if (controls.length === 0) return
  const current = controls.indexOf(document.activeElement as HTMLElement)
  const next = current === -1 ? (delta > 0 ? 0 : controls.length - 1) : Math.min(Math.max(current + delta, 0), controls.length - 1)
  focus(controls[next])
}

/** Send a key to the focused element, so gamepad input reuses the keyboard shortcuts. */
function press(key: string): void {
  const target = document.activeElement ?? document.body
  target.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true }))
}

function adjustRange(input: HTMLInputElement, direction: 1 | -1): void {
  if (direction > 0) input.stepUp()
  else input.stepDown()
  input.dispatchEvent(new Event('input', { bubbles: true }))
}

/**
 * Navigate a menu-like container: the D-pad moves focus (left and right adjust sliders),
 * confirm activates, and cancel/menu run the container's own handlers.
 */
function navigate(container: Element, action: GamepadAction, handlers: { cancel: () => void; menu?: () => void }): void {
  const active = document.activeElement as HTMLElement | null
  const focused = active && container.contains(active) ? active : null
  const slider = focused instanceof HTMLInputElement && focused.type === 'range' ? focused : null

  switch (action) {
    case 'up':
      return moveFocus(container, -1)
    case 'down':
      return moveFocus(container, 1)
    case 'left':
    case 'prev':
      return slider ? adjustRange(slider, -1) : moveFocus(container, -1)
    case 'right':
    case 'next':
      return slider ? adjustRange(slider, 1) : moveFocus(container, 1)
    case 'confirm':
      if (!focused) return moveFocus(container, 1)
      // Text fields take focus; typing is left to the platform's on-screen keyboard.
      if (focused instanceof HTMLTextAreaElement || (focused instanceof HTMLInputElement && focused.type === 'text')) return
      if (!slider) focused.click()
      return
    case 'cancel':
      return handlers.cancel()
    case 'menu':
      return (handlers.menu ?? handlers.cancel)()
    case 'references':
      return press('r')
  }
}

/** Act on a gamepad action according to what's on screen. */
export function routeGamepadAction(action: GamepadAction): void {
  const dialog = document.querySelector<HTMLDialogElement>('dialog[open]')
  if (dialog) {
    return navigate(dialog, action, {
      // Back out of a submenu first; close from the top level.
      cancel: () => {
        const back = dialog.querySelector<HTMLElement>('.tome-menu-back')
        if (back) back.click()
        else dialog.close()
      },
      menu: () => dialog.close(),
    })
  }

  const sidebar = document.querySelector('.tome-sidebar')
  if (sidebar?.contains(document.activeElement)) {
    return navigate(sidebar, action, {
      // From an entry back to the page's list; from the list, close the sidebar.
      cancel: () => {
        const back = sidebar.querySelector<HTMLElement>('.tome-sidebar-back')
        if (back) back.click()
        else press('Escape')
      },
      menu: () => press('Escape'),
    })
  }

  const landing = document.querySelector('.tome-landing')
  if (landing) return navigate(landing, action, { cancel: () => {} })

  if (!document.querySelector('.tome-reading')) return
  switch (action) {
    case 'confirm':
    case 'next':
    case 'right':
      return press('ArrowRight')
    case 'cancel':
    case 'prev':
    case 'left':
      return press('ArrowLeft')
    case 'menu':
      return press('Escape')
    case 'references':
      return press('r')
    case 'illustration':
      return press('i')
    case 'up':
    case 'down': {
      // Long pages scroll; the D-pad moves through them a portion of the screen at a time.
      const reader = document.querySelector('.tome-reader')
      reader?.scrollBy({ top: (action === 'down' ? 1 : -1) * reader.clientHeight * 0.4, behavior: 'smooth' })
    }
  }
}
