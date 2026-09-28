/**
 * Finding a command in the palette by what it is called.
 *
 * The palette's works, drafts and notes are found by the backend, over bodies
 * the window never holds. Its actions and screens are the window's own - a
 * dozen names, already translated - so they are matched here, against the
 * words on screen, in whichever language the window speaks.
 */

/** Anything the palette lists under a name of its own. */
export interface Named {
  label: string
}

/** A label cut into words, whatever separates them: spaces, quotes, dashes. */
const words = (text: string) => text.split(/[^\p{L}\p{N}]+/u).filter((word) => word !== '')

/**
 * How well `label` answers the query, best first; `null` when it does not.
 *
 * Every term has to be in the label, in any order: "theme dark" finds "Switch
 * to the dark theme". Then a label that begins with what was typed comes
 * first, a label where every term begins a word next, and one that merely
 * contains the letters last - "cal" is the Calendar long before it is
 * "Critical".
 */
function rank(label: string, terms: string[]): number | null {
  const text = label.toLowerCase()
  if (!terms.every((term) => text.includes(term))) return null
  if (text.startsWith(terms.join(' '))) return 0
  const starts = words(text)
  if (terms.every((term) => starts.some((word) => word.startsWith(term)))) return 1
  return 2
}

/**
 * The commands that match `query`, best first.
 *
 * An empty query matches everything, in the order given: the palette opened
 * on nothing is a list of what can be done, not a list of nothing. The sort
 * is stable, so commands that match equally well keep the order the palette
 * put them in - it does not shuffle under the cursor as the query grows.
 */
export function matchCommands<T extends Named>(query: string, commands: readonly T[]): T[] {
  const terms = words(query.toLowerCase())
  if (terms.length === 0) return [...commands]

  return commands
    .map((command) => ({ command, rank: rank(command.label, terms) }))
    .filter((entry): entry is { command: T; rank: number } => entry.rank !== null)
    .sort((left, right) => left.rank - right.rank)
    .map((entry) => entry.command)
}
