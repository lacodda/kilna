import type { LineRow, ShelfView } from '@/lib/api/types'

/*
 * The phrases as the window reads them (ADR 0064): which phrases a part of
 * the screen shows, in what order, and how a typed list becomes phrases.
 *
 * A phrase is a note of a kind kept as one line, and a shelf is a list of
 * note ids in the order they were put there. Everything here is a pure
 * function of those two lists, so the screen and its tests read the same
 * answer.
 */

/** The part of the phrases on show: `ALL_LINES`, `NO_SHELF`, or one shelf by
 *  its id. Shelf ids are UUIDs, so neither word can be taken for one. */
export type LineScope = string

export const ALL_LINES = 'all'
export const NO_SHELF = 'none'

/** The shelf a scope is, when it is one. */
export function shelfOf(scope: LineScope, shelves: readonly ShelfView[]): ShelfView | null {
  return shelves.find((shelf) => shelf.id === scope) ?? null
}

/**
 * The phrases a scope shows, out of the rows the screen's filters left. A
 * shelf keeps the order its phrases were put there, which is the owner's; all
 * of them, and those on no shelf, keep the order they came in - the latest
 * touched first. A shelf's id the rows do not have - a phrase filtered out,
 * or deleted a moment ago - is passed over rather than drawn empty.
 */
export function linesIn(
  scope: LineScope,
  rows: readonly LineRow[],
  shelves: readonly ShelfView[],
): LineRow[] {
  const shelf = shelfOf(scope, shelves)
  if (shelf !== null) {
    const byId = new Map(rows.map((row) => [row.id, row]))
    return shelf.note_ids.flatMap((id) => {
      const row = byId.get(id)
      return row === undefined ? [] : [row]
    })
  }
  if (scope === NO_SHELF) return rows.filter((row) => row.shelf_ids.length === 0)
  return [...rows]
}

/**
 * Lines as a person types or pastes them: one, or a list by lines. Not by
 * commas - "coffee, a pulsar and the sea" is one phrase. A line given twice
 * is kept once, the first spelling winning; the backend matches what is kept
 * whatever the case, and asking it for the same line twice in one list would
 * only say "already there" about the list itself.
 */
export function splitLines(text: string): string[] {
  const seen = new Set<string>()
  const lines: string[] = []
  for (const part of text.split(/[\n\r]+/)) {
    const line = part.trim()
    const key = line.toLocaleLowerCase()
    if (line === '' || seen.has(key)) continue
    seen.add(key)
    lines.push(line)
  }
  return lines
}

/** How a phrase's word is sung, as the row says it under the phrase: the
 *  sung spelling alone where it is the phrase's own word, "written → sung"
 *  where it is another form. */
export function sungOf(row: Pick<LineRow, 'body' | 'sung'>): string {
  const own = row.body.trim().toLocaleLowerCase()
  return row.sung
    .map((one) =>
      one.written.trim().toLocaleLowerCase() === own ? one.sung : `${one.written} → ${one.sung}`,
    )
    .join(', ')
}
