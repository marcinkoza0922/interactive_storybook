<script lang="ts">
  import type { Bundle } from '../bundle/types'
  import { search, type SearchMatch } from '../search/search'

  interface Props {
    bundle: Bundle
    /** The last chapter searched: the furthest reached, or the end once the book has been read. */
    lastChapter: number
    query: string
    onfind: (match: SearchMatch) => void
  }

  let { bundle, lastChapter, query = $bindable(), onfind }: Props = $props()

  const LIMIT = 200
  const matches = $derived(search(bundle, query, lastChapter, LIMIT))
  /** Matches come in reading order, so each chapter's are together. */
  const groups = $derived(
    matches.reduce<{ chapter: number; matches: SearchMatch[] }[]>((groups, match) => {
      const last = groups.at(-1)
      if (last?.chapter === match.chapter) last.matches.push(match)
      else groups.push({ chapter: match.chapter, matches: [match] })
      return groups
    }, []),
  )
  const wholeBook = $derived(lastChapter >= bundle.chapters.length - 1)

  function summary(): string {
    if (matches.length === 0) return 'No matches in the chapters you’ve reached.'
    if (matches.length >= LIMIT) return `The first ${LIMIT} matches. Add words to narrow the search.`
    return matches.length === 1 ? '1 match' : `${matches.length} matches`
  }

  function place(match: SearchMatch): string {
    const page = `Page ${match.page + 1}`
    return match.target.kind === 'footnote' ? `${page} · footnote ${match.target.number}` : page
  }

  /** Enter goes to the first match. */
  function onkeydown(event: KeyboardEvent) {
    if (event.key !== 'Enter' || !matches[0]) return
    event.preventDefault()
    onfind(matches[0])
  }
</script>

<h2 class="tome-menu-heading">Search</h2>
<input
  class="tome-input"
  type="search"
  aria-label="Search the text"
  aria-describedby="tome-search-scope"
  placeholder="A word or phrase"
  autocomplete="off"
  spellcheck="false"
  data-autofocus
  bind:value={query}
  {onkeydown}
/>
<p class="tome-menu-meta" id="tome-search-scope">
  {wholeBook
    ? 'Searching the whole book.'
    : `Searching up to ${bundle.chapters[lastChapter].title}. Chapters you haven’t reached aren’t searched.`}
</p>

<p class="tome-menu-meta" role="status">{query.trim() ? summary() : ''}</p>
{#each groups as group (group.chapter)}
  <h3 class="tome-search-chapter">{bundle.chapters[group.chapter].title}</h3>
  <ul class="tome-menu-list">
    {#each group.matches as match, i (i)}
      <li>
        <button class="tome-menu-item tome-search-result" onclick={() => onfind(match)}>
          <span class="tome-menu-meta">{place(match)}</span>
          <span class="tome-search-snippet">
            {match.snippet.before}<mark>{match.snippet.match}</mark>{match.snippet.after}
          </span>
        </button>
      </li>
    {/each}
  </ul>
{/each}
