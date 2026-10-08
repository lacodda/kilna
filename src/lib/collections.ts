import type { Collection, ScoredWork } from '@/lib/api/types'
import { daysBetween } from '@/lib/readiness'

/*
 * What a collection's card and its open page say about it, worked out once
 * from the collection and the catalogue's rows - so the card in the grid and
 * the summary beside the list cannot read the same album two ways.
 */

/** How far a collection is toward what it was meant to hold. */
export interface Progress {
  /** How many works it holds. */
  held: number
  /** How many it is meant to hold, when someone said. */
  target: number | null
  /** The share of the target held, 0 to 1 - a twelve-track album holding
   *  fourteen is full, not 117% of one. Null with no target: a bar with
   *  nothing to fill towards would be a number made up for it. */
  fraction: number | null
  /** Works still to come; 0 once the target is met. Null with no target. */
  left: number | null
  met: boolean
  /** The day it is meant to be done by. */
  due: string | null
  /** Whole days from today to that day, negative once it has passed. */
  daysLeft: number | null
}

export function progressOf(collection: Collection, today: string): Progress {
  const held = collection.work_ids.length
  const target = collection.target_size
  const met = target !== null && held >= target
  return {
    held,
    target,
    fraction: target === null ? null : Math.min(1, held / target),
    left: target === null ? null : Math.max(0, target - held),
    met,
    due: collection.due_on,
    daysLeft: collection.due_on === null ? null : daysBetween(today, collection.due_on),
  }
}

/** Where the works of a collection stand as a whole, by their scores. */
export interface Standing {
  /** Its works, in its order - those the catalogue knows. */
  works: ScoredWork[]
  /** How many of them have been judged. */
  scored: number
  /** The mean total of the judged ones, or null while none is. */
  average: number | null
  /** The lowest judged work - what holds the album back - once there are two
   *  to compare: the only judged work is not weaker than anything. */
  weakest: ScoredWork | null
  /** The highest, on the same terms. */
  strongest: ScoredWork | null
}

/**
 * The verdict of a collection, from its works' own.
 *
 * Read from the catalogue's rows, which carry each work's speaking total: a
 * second sum of scores kept on the collection would be one more number that
 * could fall behind the works it is made of. An unjudged work counts towards
 * nothing - it is not a zero, it is a question nobody has answered, and the
 * card says how many there are.
 */
export function standingOf(collection: Collection, rows: readonly ScoredWork[]): Standing {
  const byId = new Map(rows.map((row) => [row.work_id, row]))
  const works = collection.work_ids.flatMap((id) => {
    const row = byId.get(id)
    return row === undefined ? [] : [row]
  })
  const judged = works.filter((row): row is ScoredWork & { total: number } => row.total !== null)
  const average =
    judged.length === 0 ? null : judged.reduce((sum, row) => sum + row.total, 0) / judged.length

  let weakest: ScoredWork | null = null
  let strongest: ScoredWork | null = null
  if (judged.length > 1) {
    // The first of equals, in the album's order: ties go to the earlier track,
    // so the answer does not change between two renders of the same list.
    for (const row of judged) {
      if (weakest === null || row.total < (weakest.total ?? Infinity)) weakest = row
      if (strongest === null || row.total > (strongest.total ?? -Infinity)) strongest = row
    }
    // Every judged work the same: nothing is weaker or stronger than the rest.
    if (weakest?.total === strongest?.total) {
      weakest = null
      strongest = null
    }
  }

  return { works, scored: judged.length, average, weakest, strongest }
}

/** The list with `id` moved to index `to` of the result; unchanged when `id`
 *  is not in it. `to` is clamped, as a column's move is. */
export function moveTo(order: readonly string[], id: string, to: number): string[] {
  const from = order.indexOf(id)
  if (from === -1) return [...order]
  const without = order.filter((one) => one !== id)
  const at = Math.max(0, Math.min(without.length, to))
  return [...without.slice(0, at), id, ...without.slice(at)]
}
