import type { PromptTemplate } from '@/lib/api'

/**
 * Choosing a profile action by typing rather than by aiming.
 *
 * The action buttons above the composer work until a profile has more than a
 * handful; then they are a wall to read every time. Typing `/` and two letters
 * is the same choice made without leaving the keyboard — and it costs nothing
 * to a profile with three actions, because the buttons stay.
 */

/** What the palette is doing right now. */
export interface Palette {
  /** What was typed after the slash, lowercased. */
  query: string
  /** Actions still matching, best first. */
  matches: PromptTemplate[]
}

/**
 * Read a composer's draft as a palette query, or decide it is not one.
 *
 * The palette opens only on a draft that is *nothing but* the slash command:
 * a `/` inside a sentence is a slash — dates, paths and "and/or" all contain
 * one, and stealing the keyboard from someone writing about a file path would
 * be worse than not having a palette at all.
 */
export function reading(draft: string, actions: PromptTemplate[]): Palette | null {
  if (!draft.startsWith('/')) return null

  const query = draft.slice(1)
  // A newline means the draft has grown past being a command.
  if (query.includes('\n')) return null

  return { query: query.toLowerCase(), matches: matching(query, actions) }
}

/**
 * Actions matching what has been typed, best first.
 *
 * Matched against both the label and the key, because either is what someone
 * remembers: the label is what the button says, the key is what the profile
 * document calls it.
 */
export function matching(query: string, actions: PromptTemplate[]): PromptTemplate[] {
  const needle = query.trim().toLowerCase()
  if (needle === '') return actions

  const scored = actions
    .map((action) => ({ action, rank: rank(action, needle) }))
    .filter((entry) => entry.rank !== null)

  // Stable within a rank, so equally good matches keep the profile's own order
  // rather than an arbitrary one that shifts as the query grows.
  scored.sort((left, right) => (left.rank ?? 0) - (right.rank ?? 0))

  return scored.map((entry) => entry.action)
}

/**
 * How well an action matches, lower being better; null when it does not.
 *
 * Matching is by *word starts only*, never by substring anywhere. Typing "cri"
 * has to mean "Critique" — a plain `includes` also matches "Des-cri-be the
 * style", and a palette that offers the wrong action for an obvious query is
 * worse than no palette. Measured on the seeded music profile, which is exactly
 * where that pair collides.
 *
 * The whole label starting with the query beats a later word starting with it,
 * so the action whose name begins the way you typed comes first.
 */
function rank(action: PromptTemplate, needle: string): number | null {
  const label = action.label.toLowerCase()
  const key = action.key.toLowerCase()

  if (label.startsWith(needle) || key.startsWith(needle)) return 0

  // Any later word of the label, so "the lyrics" is reachable by typing "lyr".
  if (label.split(/\s+/).some((word) => word.startsWith(needle))) return 1

  return null
}
