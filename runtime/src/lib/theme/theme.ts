import type { Background, Backgrounds, Bundle, Decoration, FontFile, LandingTheme, Paper } from '../bundle/types'

/** The theme in force at one point in the book, overrides applied. */
export interface EffectiveTheme {
  tokens: Record<string, string>
  fonts: FontFile[]
  styles: Record<string, Record<string, string>>
  papers: Record<string, Paper>
  backgrounds: Backgrounds
  decoration: Decoration
  landing: LandingTheme
  custom_css?: string
}

/**
 * The theme for a chapter: the base theme plus every override from that chapter or earlier,
 * in chapter order. Overrides merge per token, style, background and decoration field;
 * null clears a background or decoration. `chapter` null means before reading starts.
 */
export function effectiveTheme(bundle: Bundle, chapter: number | null): EffectiveTheme {
  const base = bundle.theme ?? {}
  const theme: EffectiveTheme = {
    tokens: { ...base.tokens },
    fonts: base.fonts ?? [],
    styles: { ...base.styles },
    papers: base.papers ?? {},
    backgrounds: { ...base.backgrounds },
    decoration: { ...base.decoration },
    landing: base.landing ?? {},
    custom_css: base.custom_css,
  }
  if (chapter === null) return theme

  const order = new Map(bundle.chapters.map((c, i) => [c.id, i]))
  const applicable = (base.overrides ?? [])
    .map((override) => ({ override, at: order.get(override.from) ?? Infinity }))
    .filter(({ at }) => at <= chapter)
    .sort((a, b) => a.at - b.at)

  for (const { override } of applicable) {
    Object.assign(theme.tokens, override.tokens)
    Object.assign(theme.styles, override.styles)
    Object.assign(theme.backgrounds, override.backgrounds)
    Object.assign(theme.decoration, override.decoration)
  }
  return theme
}

// Theme values are written into a stylesheet; these would let a value escape its declaration.
const UNSAFE_VALUE = /[;{}<>]/
const TOKEN_NAME = /^[a-z0-9]+(-[a-z0-9]+)*$/
const PROPERTY_NAME = /^(--)?[a-z][a-z0-9-]*$/

function safe(value: string, where: string): boolean {
  if (!UNSAFE_VALUE.test(value)) return true
  console.warn(`Ignoring theme value for ${where}: it may not contain ; { } < or >`)
  return false
}

const quote = (url: string) => JSON.stringify(url)

/** Rewrite relative url(...) references so they resolve against book.json, not the page. */
function resolveUrls(value: string, url: (src: string) => string): string {
  return value.replace(/url\(\s*(['"]?)([^'")]+)\1\s*\)/g, (_, _quote, path: string) => `url(${quote(url(path.trim()))})`)
}

export function backgroundKind(background: Background): 'image' | 'animated' | 'video' | null {
  if (!background.src) return null
  if (background.kind) return background.kind
  if (/\.(mp4|webm|ogv|mov)$/i.test(background.src)) return 'video'
  if (/\.gif$/i.test(background.src)) return 'animated'
  return 'image'
}

/**
 * Paper grain as a tiling SVG noise image, so authors don't need a texture file. The noise
 * is black at an opacity set by `amount` (0–1), which darkens any paper color evenly.
 */
export function grainImage(amount: number): string {
  const alpha = Math.min(Math.max(amount, 0), 1) * 0.5
  if (alpha === 0) return 'none'
  const svg =
    `<svg xmlns='http://www.w3.org/2000/svg' width='240' height='240'>` +
    `<filter id='g'><feTurbulence type='fractalNoise' baseFrequency='0.8' numOctaves='3' stitchTiles='stitch'/>` +
    `<feColorMatrix values='0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  0 0 0 ${alpha.toFixed(3)} 0'/></filter>` +
    `<rect width='100%' height='100%' filter='url(#g)'/></svg>`
  return `url("data:image/svg+xml,${encodeURIComponent(svg)}")`
}

function paperTokens(paper: Paper, url: (src: string) => string): string[] {
  const tokens: string[] = []
  if (paper.color !== undefined && safe(paper.color, 'a paper color')) tokens.push(`--tome-paper: ${paper.color};`)
  if (paper.texture !== undefined) tokens.push(`--tome-page-texture: ${paper.texture ? `url(${quote(url(paper.texture))})` : 'none'};`)
  if (paper.grain !== undefined) tokens.push(`--tome-paper-grain: ${grainImage(paper.grain)};`)
  return tokens
}

/** The generated stylesheet for a theme: font faces, token values and named styles. */
export function themeCss(theme: EffectiveTheme, url: (src: string) => string): string {
  const rules: string[] = []

  for (const font of theme.fonts) {
    if (!safe(font.family, 'a font family')) continue
    rules.push(
      `@font-face { font-family: ${quote(font.family)}; src: url(${quote(url(font.src))}); ` +
        `font-weight: ${font.weight ?? 'normal'}; font-style: ${font.style ?? 'normal'}; font-display: swap; }`,
    )
  }

  const tokens: string[] = []
  for (const [name, value] of Object.entries(theme.tokens)) {
    if (!TOKEN_NAME.test(name)) console.warn(`Ignoring theme token "${name}": names are lowercase words joined by -`)
    else if (safe(value, `token "${name}"`)) tokens.push(`--tome-${name}: ${resolveUrls(value, url)};`)
  }

  const { page_texture, page_frame, chapter_ornament, paper_grain } = theme.decoration
  if (page_texture) tokens.push(`--tome-page-texture: url(${quote(url(page_texture))});`)
  if (paper_grain !== undefined) tokens.push(`--tome-paper-grain: ${grainImage(paper_grain)};`)
  if (chapter_ornament) tokens.push(`--tome-chapter-ornament: url(${quote(url(chapter_ornament))});`)
  if (page_frame && safe(page_frame.width, 'the page frame width')) {
    const repeat = page_frame.repeat ?? 'round'
    tokens.push(
      `--tome-page-frame: url(${quote(url(page_frame.src))}) ${Number(page_frame.slice)} / ${page_frame.width} ${repeat};`,
      `--tome-page-frame-width: ${page_frame.width};`,
    )
  }
  if (tokens.length > 0) rules.push(`:root { ${tokens.join(' ')} }`)

  // Named papers apply to the pages that ask for them.
  for (const [name, paper] of Object.entries(theme.papers)) {
    if (!TOKEN_NAME.test(name)) continue
    rules.push(`.tome-reading[data-paper=${quote(name)}] { ${paperTokens(paper, url).join(' ')} }`)
  }

  for (const [name, declarations] of Object.entries(theme.styles)) {
    if (!TOKEN_NAME.test(name)) continue
    const body = Object.entries(declarations)
      .filter(([property, value]) => PROPERTY_NAME.test(property) && safe(value, `style "${name}"`))
      .map(([property, value]) => `${property}: ${resolveUrls(value, url)};`)
    rules.push(`[data-tome-style=${quote(name)}] { ${body.join(' ')} }`)
  }

  return rules.join('\n')
}
