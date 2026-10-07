import type { Lineage } from '@/lib/history'

/**
 * The tree of a role's versions, drawn beside the list of them (ADR 0055).
 *
 * The list stays a list - newest first, one row per revision, the arrows
 * walking it - and a narrow gutter beside it draws which revision was written
 * from which, the way a git client draws its graph: a dot per row, a line
 * from each version down to the one it was written from, and a second column
 * where two versions were written from the same one. A history with no
 * recorded lineage has no lines to draw, and gets no gutter: dots on their
 * own would only say "unknown" twenty times.
 */

/** What the gutter draws beside one row. */
export interface GraphRow {
  /** The column the row's dot sits in, from 0 at the left. */
  column: number
  /** A line comes into the dot from above, in its own column: a newer
   *  version, higher in the list, was written from this one. */
  up: boolean
  /** A line leaves the dot downwards, to the version it was written from. */
  down: boolean
  /** Columns whose lines pass this row by without touching its dot. */
  through: number[]
  /** Columns whose lines end at this dot from above - versions in other
   *  columns, written from this one. */
  joins: number[]
}

export interface Graph {
  /** How many columns the gutter needs. */
  columns: number
  rows: GraphRow[]
}

/**
 * The graph for a newest-first list of one role's versions, or `null` when
 * no version in it names another of it as its parent.
 *
 * A parent always stands below its child - it was written first, so its
 * revision is lower - and a parent the list does not hold (pruned, or above
 * its child in a list ordered some other way) is no line at all.
 */
export function graphOf<T extends Lineage>(versions: readonly T[]): Graph | null {
  const row = new Map(versions.map((version, index) => [version.id, index]))
  const parentRow = (version: T, index: number): number | null => {
    const at = version.parent_version_id === null ? undefined : row.get(version.parent_version_id)
    return at !== undefined && at > index ? at : null
  }
  if (!versions.some((version, index) => parentRow(version, index) !== null)) return null

  // Per column, the version a line in it is on its way down to.
  const awaiting: (string | null)[] = []
  const rows: GraphRow[] = []
  let columns = 0

  versions.forEach((version, index) => {
    const incoming = awaiting.flatMap((id, column) => (id === version.id ? [column] : []))
    let column = incoming[0] ?? awaiting.indexOf(null)
    if (column === -1) {
      column = awaiting.length
      awaiting.push(null)
    }
    const joins = incoming.slice(1)
    for (const joined of joins) awaiting[joined] = null
    const through = awaiting.flatMap((id, other) =>
      id !== null && other !== column ? [other] : [],
    )

    const down = parentRow(version, index) !== null
    awaiting[column] = down ? version.parent_version_id : null
    rows.push({ column, up: incoming.length > 0, down, through, joins })
    columns = Math.max(columns, awaiting.length)

    // A column whose line has ended is free again; trailing free ones go, so
    // a branch that closed does not keep the gutter wide below it.
    while (awaiting.length > 0 && awaiting.at(-1) === null) awaiting.pop()
  })

  return { columns, rows }
}
