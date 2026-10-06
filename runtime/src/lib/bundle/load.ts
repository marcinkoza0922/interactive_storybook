import { SUPPORTED_SCHEMA_VERSION, type Bundle } from './types'

/**
 * URLs inside block and reference HTML are relative to book.json, like every other asset
 * path in the bundle. Rewrite them to absolute URLs once, so they don't resolve against
 * the page instead.
 */
function resolveHtmlUrls(html: string, base: string): string {
  if (!/\s(src|href)=/.test(html)) return html
  const template = document.createElement('template')
  template.innerHTML = html
  for (const element of template.content.querySelectorAll('[src], a[href]')) {
    for (const attribute of ['src', 'href']) {
      const value = element.getAttribute(attribute)
      if (value && !value.startsWith('#')) element.setAttribute(attribute, new URL(value, base).href)
    }
  }
  return template.innerHTML
}

export async function loadBundle(url: string): Promise<Bundle> {
  const response = await fetch(url)
  if (!response.ok) {
    throw new Error(`Could not load the book (${response.status} ${response.statusText}).`)
  }
  const bundle = (await response.json()) as Bundle

  if (bundle.bundle_schema_version !== SUPPORTED_SCHEMA_VERSION) {
    throw new Error(
      `This book was built for bundle schema ${bundle.bundle_schema_version}, ` +
        `but this runtime supports schema ${SUPPORTED_SCHEMA_VERSION}. Rebuild the book with a matching version of tome.`,
    )
  }
  if (bundle.chapters.length === 0 || bundle.chapters.some((c) => c.pages.length === 0)) {
    throw new Error('The book has no content: every chapter needs at least one page.')
  }

  const base = new URL(url, document.baseURI).href
  for (const block of bundle.chapters.flatMap((c) => c.pages.flatMap((p) => p.blocks))) {
    block.html = resolveHtmlUrls(block.html, base)
  }
  for (const section of (bundle.references ?? []).flatMap((r) => r.sections)) {
    section.html = resolveHtmlUrls(section.html, base)
  }
  return bundle
}
