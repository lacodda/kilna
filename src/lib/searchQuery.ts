/**
 * The catalogue's search box, read as a query rather than as a title.
 *
 * `winter tier:clip status:draft` narrows three ways from one line. The
 * dropdowns beside the box do the same thing with fewer keystrokes and more
 * clicks; neither replaces the other, and both write into the same
 * `CatalogueFilter`, so what one sets the other shows.
 *
 * ## Why an unknown field is not an error
 *
 * `Ratio: a love song` is a title, not a filter on a field called `ratio`. A
 * parser that rejected it would make the box refuse perfectly good text, and a
 * parser that dropped the term would lose it silently. So a term whose field is
 * not one this app knows stays free text, colon and all — the box degrades to
 * what it was before operators existed, which is the behaviour a person who
 * never learns the syntax should get.
 *
 * ## Why values are matched against labels too
 *
 * The keys are `clip`, `draft`, `song`; the screen says Clip, Draft, Song, and
 * in Russian it says something else again. Someone typing what they can see is
 * not making a mistake. Resolution goes key first, then label, so a profile
 * whose label collides with another entry's key still behaves predictably.
 */
import type { CatalogueFilter } from '@/lib/catalogue'

/** One entry of profile vocabulary: what it is called and what it is keyed by. */
export interface Term {
  key: string
  label: string
}

/** The vocabulary a query is resolved against — the active profile's own words. */
export interface Vocabulary {
  statuses: Term[]
  kinds: Term[]
  tiers: Term[]
}

/**
 * The fields the box understands.
 *
 * `tag:` is here and `mark:` is not, deliberately. A tag is the author's own
 * vocabulary and is what they would think to search by; a mark is about this
 * week and is read off the row at a glance, not hunted for. Adding `mark:`
 * later costs one entry in this list.
 */
export const FIELDS = ['status', 'kind', 'tier', 'tag'] as const
export type Field = (typeof FIELDS)[number]

const IS_FIELD = new Set<string>(FIELDS)

/**
 * Split a line into terms, keeping quoted runs whole.
 *
 * `"paper boats" tier:clip` is two terms, not three. Quotes are the only way to
 * search for a phrase, and the only way to search for text that contains a
 * space and a colon.
 */
function tokenise(query: string): string[] {
  const tokens: string[] = []
  let current = ''
  let quoted = false

  for (const character of query) {
    if (character === '"') {
      quoted = !quoted
      // A closing quote ends the term even against the next character, so
      // `"a""b"` is two terms rather than one run of letters.
      continue
    }
    if (!quoted && /\s/.test(character)) {
      if (current !== '') tokens.push(current)
      current = ''
      continue
    }
    current += character
  }

  if (current !== '') tokens.push(current)
  return tokens
}

/**
 * Which entry of a vocabulary the typed value means.
 *
 * Case-folded on both sides with `toLowerCase`, which knows Cyrillic —
 * `toLocaleLowerCase` would tie the answer to the machine's locale, and a query
 * should mean the same thing on every machine.
 */
function resolve(terms: Term[], value: string): string | undefined {
  const wanted = value.toLowerCase()
  const byKey = terms.find((term) => term.key.toLowerCase() === wanted)
  if (byKey) return byKey.key
  return terms.find((term) => term.label.toLowerCase() === wanted)?.key
}

/** What a parsed line asks for. */
export interface ParsedQuery {
  /** The filter fields the operators set. Absent fields were not mentioned. */
  filter: Pick<CatalogueFilter, 'status' | 'kind' | 'tier' | 'tag'>
  /** Everything that was not an operator, joined back into one phrase. */
  text: string
  /** Operators naming something this profile does not have, for telling the person. */
  unknown: { field: Field; value: string }[]
}

/**
 * Read the box.
 *
 * Repeating a field keeps the last one: `tier:clip tier:pic` means `pic`. One
 * value per field is what the filter holds, and the last thing typed is the
 * thing being asked for — the alternative, narrowing to nothing on a
 * contradiction, punishes a person for editing the line they are looking at.
 */
export function parseQuery(query: string, vocabulary: Vocabulary): ParsedQuery {
  const filter: ParsedQuery['filter'] = {}
  const unknown: ParsedQuery['unknown'] = []
  const free: string[] = []

  for (const token of tokenise(query)) {
    const colon = token.indexOf(':')
    const field = colon === -1 ? '' : token.slice(0, colon).toLowerCase()

    if (!IS_FIELD.has(field)) {
      free.push(token)
      continue
    }

    const value = token.slice(colon + 1)
    // `tier:` with nothing after it is someone mid-typing, not a request to
    // filter on the empty string.
    if (value === '') continue

    if (field === 'tag') {
      // Tags are free text the author invented; there is no list to check
      // against, so whatever is typed is the tag being asked for.
      filter.tag = value
      continue
    }

    const resolved = resolve(vocabularyFor(vocabulary, field as Field), value)
    if (resolved === undefined) {
      unknown.push({ field: field as Field, value })
      continue
    }

    filter[field as 'status' | 'kind' | 'tier'] = resolved
  }

  return { filter, text: free.join(' '), unknown }
}

function vocabularyFor(vocabulary: Vocabulary, field: Field): Term[] {
  switch (field) {
    case 'status':
      return vocabulary.statuses
    case 'kind':
      return vocabulary.kinds
    case 'tier':
      return vocabulary.tiers
    // Tags have no vocabulary to resolve against; the caller returns before
    // reaching here, and this keeps the switch exhaustive.
    case 'tag':
      return []
  }
}

/**
 * Write a filter back out as a line, so the box can show what the dropdowns did.
 *
 * The round trip has to hold: parsing what this produces must give the filter
 * back. That is what lets the two ways of narrowing share one state instead of
 * fighting over it.
 */
export function formatQuery(filter: CatalogueFilter): string {
  const parts: string[] = []

  for (const field of FIELDS) {
    const value = filter[field]
    if (value === undefined || value === '') continue
    parts.push(`${field}:${quote(value)}`)
  }

  const text = filter.search?.trim() ?? ''
  if (text !== '') parts.push(quote(text))

  return parts.join(' ')
}

/** A value with a space in it needs quoting, or it comes back as two terms. */
function quote(value: string): string {
  return /[\s"]/.test(value) ? `"${value.replaceAll('"', '')}"` : value
}
