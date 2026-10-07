// @vitest-environment happy-dom
import { describe, expect, it } from 'vitest'
import { prepareKinetic, TYPEWRITER_CHAR_MS } from './kinetic'
import type { Block } from './types'

function prepare(html: string, reveal?: Block['reveal']): Block {
  const block: Block = { id: 'b', html, ...(reveal ? { reveal } : {}) }
  prepareKinetic(block)
  return block
}

function parse(html: string): DocumentFragment {
  const template = document.createElement('template')
  template.innerHTML = html
  return template.content
}

describe('prepareKinetic', () => {
  it('leaves blocks without per-character effects untouched', () => {
    const html = '<p>Plain <span data-tome-rest="pulse">pulsing</span> text.</p>'
    expect(prepare(html, { step: 1, effect: 'fade' }).html).toBe(html)
  })

  it('keeps footnote markers in the animated copy out of the tab order', () => {
    const marker = '<sup><button type="button" data-tome-footnote="1">1</button></sup>'
    const dom = parse(prepare(`<p data-tome-rest="wave">Ash${marker}</p>`).html)
    expect(dom.querySelector('.tome-visual [data-tome-footnote]')!.getAttribute('tabindex')).toBe('-1')
    expect(dom.querySelector('.tome-sr-only [data-tome-footnote]')!.hasAttribute('tabindex')).toBe(false)
  })

  it('keeps an intact copy for screen readers and hides the animated one', () => {
    const html = '<p>“Oh, <span data-tome-rest="wave">wonderful</span>.”</p>'
    const dom = parse(prepare(html).html)
    expect(dom.querySelector('.tome-sr-only')!.innerHTML).toBe(html)
    expect(dom.querySelector('.tome-visual')!.getAttribute('aria-hidden')).toBe('true')
    expect(dom.querySelector('.tome-visual')!.textContent).toBe('“Oh, wonderful.”')
  })

  it('splits wave text into numbered characters grouped by word', () => {
    const dom = parse(prepare('<p><span data-tome-rest="wave">so kind</span></p>').html)
    const words = [...dom.querySelectorAll('.tome-visual .tome-word')].map((w) => w.textContent)
    expect(words).toEqual(['so', 'kind'])
    const indices = [...dom.querySelectorAll<HTMLElement>('.tome-char')].map((c) => c.style.getPropertyValue('--tome-char-index'))
    expect(indices).toEqual(['0', '1', '2', '3', '4', '5'])
  })

  it('splits inline breathe text into words only', () => {
    const dom = parse(prepare('<p>A <em data-tome-rest="breathe">slow breath</em></p>').html)
    expect(dom.querySelectorAll('.tome-word')).toHaveLength(2)
    expect(dom.querySelectorAll('.tome-char')).toHaveLength(0)
  })

  it('numbers every typewriter character, including inside markup, and sets a default pace', () => {
    const block = prepare('<p>Hi <em>you</em></p>', { step: 1, effect: 'typewriter' })
    const dom = parse(block.html)
    const chars = [...dom.querySelectorAll('.tome-visual .tome-tw')].map((c) => c.textContent)
    expect(chars).toEqual(['H', 'i', ' ', 'y', 'o', 'u'])
    expect(dom.querySelector('.tome-visual em .tome-tw')).not.toBeNull()
    expect((dom.querySelector('.tome-visual') as HTMLElement).style.getPropertyValue('--tome-tw-count')).toBe('6')
    expect(block.reveal!.duration_ms).toBe(6 * TYPEWRITER_CHAR_MS)
  })

  it('keeps an author-set typewriter duration', () => {
    expect(prepare('<p>Hello</p>', { step: 1, effect: 'typewriter', duration_ms: 2000 }).reveal!.duration_ms).toBe(2000)
  })
})
