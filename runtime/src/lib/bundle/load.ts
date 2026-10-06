import { SUPPORTED_SCHEMA_VERSION, type Bundle } from './types'

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
  return bundle
}
