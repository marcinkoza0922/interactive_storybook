import { afterEach, describe, expect, it, vi } from 'vitest'
import type { Bundle } from '../bundle/types'
import { backgroundKind, effectiveTheme, grainImage, themeCss } from './theme'

const chapters = ['one', 'two', 'three'].map((id) => ({ id, title: id, content_hash: 'h', pages: [{ blocks: [], words: 0 }] }))

const bundle: Bundle = {
  bundle_schema_version: 2,
  book: { id: 't', title: 'T', author: 'A', language: 'en' },
  chapters,
  theme: {
    tokens: { accent: 'red', 'page-bg': 'white' },
    styles: { handwriting: { 'font-family': 'Caveat' } },
    backgrounds: { landing: { src: 'fog.webm' }, reading: { src: 'paper.png' } },
    decoration: { chapter_ornament: 'ornament.svg', drop_caps: true },
    overrides: [
      // Deliberately out of order: overrides apply in chapter order.
      { from: 'three', tokens: { accent: 'gold' }, decoration: { drop_caps: false } },
      { from: 'two', tokens: { accent: 'green', 'page-bg': 'black' }, backgrounds: { reading: null } },
      { from: 'missing', tokens: { accent: 'never' } },
    ],
  },
}

const url = (src: string) => `https://book.test/${src}`

describe('effectiveTheme', () => {
  it('uses the base theme before reading starts and in the first chapter', () => {
    expect(effectiveTheme(bundle, null).tokens).toEqual({ accent: 'red', 'page-bg': 'white' })
    expect(effectiveTheme(bundle, 0).tokens.accent).toBe('red')
  })

  it('applies overrides from the current chapter and earlier, in chapter order', () => {
    const two = effectiveTheme(bundle, 1)
    expect(two.tokens).toEqual({ accent: 'green', 'page-bg': 'black' })
    expect(two.backgrounds.reading).toBeNull()
    expect(two.backgrounds.landing?.src).toBe('fog.webm')

    const three = effectiveTheme(bundle, 2)
    expect(three.tokens).toEqual({ accent: 'gold', 'page-bg': 'black' })
    expect(three.decoration).toEqual({ chapter_ornament: 'ornament.svg', drop_caps: false })
  })

  it('does not mutate the bundle theme', () => {
    effectiveTheme(bundle, 2)
    expect(bundle.theme!.tokens!.accent).toBe('red')
  })
})

describe('themeCss', () => {
  afterEach(() => vi.restoreAllMocks())

  it('writes tokens, decoration, fonts and named styles with resolved URLs', () => {
    const css = themeCss(
      {
        ...effectiveTheme(bundle, 0),
        tokens: { accent: 'red', 'page-surface': 'url(img/paper.png) repeat' },
        fonts: [{ family: 'Caveat', src: 'fonts/caveat.woff2' }],
        decoration: { page_frame: { src: 'frame.svg', slice: 30, width: '24px' } },
      },
      url,
    )
    expect(css).toContain('@font-face { font-family: "Caveat"; src: url("https://book.test/fonts/caveat.woff2");')
    expect(css).toContain('--tome-accent: red;')
    expect(css).toContain('--tome-page-surface: url("https://book.test/img/paper.png") repeat;')
    expect(css).toContain('--tome-page-frame: url("https://book.test/frame.svg") 30 / 24px round;')
    expect(css).toContain('[data-tome-style="handwriting"] { font-family: Caveat; }')
  })

  it('drops values and names that could escape their declaration', () => {
    vi.spyOn(console, 'warn').mockImplementation(() => {})
    const css = themeCss(
      {
        ...effectiveTheme(bundle, 0),
        tokens: { accent: 'red; } body { display: none', 'Bad Name': 'blue', ok: 'green' },
        styles: { evil: { 'color: red; }': 'x', color: 'blue' } },
      },
      url,
    )
    expect(css).not.toContain('display: none')
    expect(css).not.toContain('Bad Name')
    expect(css).toContain('--tome-ok: green;')
    expect(css).toContain('[data-tome-style="evil"] { color: blue; }')
  })
})

describe('paper', () => {
  it('writes the book paper and named papers as tokens', () => {
    const css = themeCss(
      {
        ...effectiveTheme(bundle, 0),
        decoration: { paper_grain: 0.4 },
        papers: { letter: { color: '#efe2c4', texture: 'parchment.png', grain: 0 }, plain: { texture: null } },
      },
      url,
    )
    expect(css).toContain('--tome-paper-grain: url("data:image/svg+xml,')
    expect(css).toContain(
      '.tome-reading[data-paper="letter"] { --tome-paper: #efe2c4; --tome-page-texture: url("https://book.test/parchment.png"); --tome-paper-grain: none; }',
    )
    expect(css).toContain('.tome-reading[data-paper="plain"] { --tome-page-texture: none; }')
  })

  it('generates no grain at zero and stays safe to embed in CSS', () => {
    expect(grainImage(0)).toBe('none')
    expect(grainImage(0.5)).not.toMatch(/[;{}<>]/)
  })
})

describe('backgroundKind', () => {
  it('infers the kind from the file extension', () => {
    expect(backgroundKind({ src: 'fog.webm' })).toBe('video')
    expect(backgroundKind({ src: 'rain.GIF' })).toBe('animated')
    expect(backgroundKind({ src: 'paper.png' })).toBe('image')
    expect(backgroundKind({ src: 'loop.png', kind: 'animated' })).toBe('animated')
    expect(backgroundKind({ color: '#222' })).toBeNull()
  })
})
