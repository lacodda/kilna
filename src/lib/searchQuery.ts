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
  /** The stage stops, each carrying the percentage the filter is written in. */
  stages: (Term & { percent: number })[]
  /** The collections, keyed by id and named by title. The filter holds the
   *  id; a person types the name, and the line shows the name back. */
  collections: Term[]
}

/**
 * The fields the box understands.
 *
 * `tag:` is here and `mark:` is not, deliberately. A tag is the author's own
 * vocabulary and is what they would think to search by; a mark is about this
 * week and is read off the row at a glance, not hunted for. Adding `mark:`
 * later costs one entry in this list.
 *
 * `collection:` arrived with the collections screen (v0.92) and not before:
 * narrowing by a thing nobody could see or make would have promised it twice.
 */
export const FIELDS = ['status', 'kind', 'tier', 'tag', 'stage', 'collection'] as const
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
  filter: Pick<CatalogueFilter, 'status' | 'kind' | 'tier' | 'tag' | 'stage' | 'collection'>
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

    if (field === 'stage') {
      // A stop is named by its key or its label like any other vocabulary, but
      // the filter holds the percentage it stands at - so the resolution is the
      // same and the value that comes out of it is not a key.
      // A bare number first, because that is what `formatQuery` writes and the
      // round trip has to hold on a machine whose profile names the stops
      // differently from the one the line was written on.
      const asNumber = /^\d{1,3}$/.test(value) ? Number(value) : undefined
      const stop =
        asNumber !== undefined
          ? vocabulary.stages.find((entry) => entry.percent === asNumber)
          : vocabulary.stages.find(
              (entry) =>
                entry.key.toLowerCase() === value.toLowerCase() ||
                entry.label.toLowerCase() === value.toLowerCase(),
            )
      if (stop === undefined) {
        unknown.push({ field: 'stage', value })
      } else {
        filter.stage = stop.percent
      }
      continue
    }

    const resolved = resolve(vocabularyFor(vocabulary, field as Field), value)
    if (resolved === undefined) {
      unknown.push({ field: field as Field, value })
      continue
    }

    filter[field as 'status' | 'kind' | 'tier' | 'collection'] = resolved
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
    case 'collection':
      return vocabulary.collections
    // Tags and stages are resolved by the caller - a tag against nothing, a
    // stage into a number rather than a key - and return before reaching here.
    // Listed so the switch stays exhaustive.
    case 'tag':
    case 'stage':
      return []
  }
}

/**
 * Write a filter back out as a line, so the box can show what the dropdowns did.
 *
 * The round trip has to hold: parsing what this produces must give the filter
 * back. That is what lets the two ways of narrowing share one state instead of
 * fighting over it.
 *
 * A collection is held by id and written by its name, which is what a person
 * reads and would type - `collection:"Deep time"`, not a UUID. Its id stands
 * in only when the name would not come back to the same collection: before
 * the collections have arrived, or when two share a name.
 */
export function formatQuery(
  filter: CatalogueFilter,
  names: Pick<Vocabulary, 'collections'> = { collections: [] },
): string {
  const parts: string[] = []

  for (const field of FIELDS) {
    if (field === 'stage') continue
    const value = field === 'collection' ? collectionWord(filter.collection, names) : filter[field]
    if (value === undefined || value === '') continue
    parts.push(`${field}:${quote(value)}`)
  }

  // Written back as the number it is stored as, so the round trip holds without
  // the profile: `stage:80` parses to 80 whatever that stop is called.
  if (filter.stage !== undefined) parts.push(`stage:${filter.stage}`)

  const text = filter.search?.trim() ?? ''
  if (text !== '') parts.push(quote(text))

  return parts.join(' ')
}

/** The word a collection is written by: its name when the name finds it
 *  again, its id otherwise. */
function collectionWord(
  id: string | undefined,
  names: Pick<Vocabulary, 'collections'>,
): string | undefined {
  if (id === undefined) return undefined
  const named = names.collections.find((term) => term.key === id)
  // A quote cannot be written inside a quoted value, so a name that holds one
  // would come back as another name.
  if (named === undefined || named.label.includes('"')) return id
  return resolve(names.collections, named.label) === id ? named.label : id
}

/** A value with a space in it needs quoting, or it comes back as two terms. */
function quote(value: string): string {
  return /[\s"]/.test(value) ? `"${value.replaceAll('"', '')}"` : value
}
