import type { Move, Placement, ScheduledRelease } from '@/lib/api/types'

/** A planned-but-unbooked chip: where a queued release would land. */
export interface Ghost {
  releaseId: string
  workId: string
  title: string
  kind: string
}

/**
 * Join an auto-layout plan with the queue it was drawn from, grouped by day
 * for the grid.
 *
 * A placement whose release is not in the queue anymore is dropped rather than
 * drawn blank: the plan is already stale, the backend will refuse it on apply,
 * and a ghost with no name answers no question in the meantime.
 */
export function ghostsOf(
  placements: readonly Placement[],
  queue: readonly ScheduledRelease[],
): Map<string, Ghost[]> {
  const byRelease = new Map(queue.map((entry) => [entry.id, entry]))
  const ghosts = new Map<string, Ghost[]>()

  for (const placement of placements) {
    const entry = byRelease.get(placement.release_id)
    if (entry === undefined) continue

    const ghost: Ghost = {
      releaseId: entry.id,
      workId: entry.work_id,
      title: entry.work_title,
      kind: entry.kind,
    }
    const day = ghosts.get(placement.date)
    if (day === undefined) ghosts.set(placement.date, [ghost])
    else day.push(ghost)
  }

  return ghosts
}

/**
 * A shift waiting for an answer, grouped by the day each release would land
 * on: the booked releases it moves, drawn as ghosts where they would go.
 *
 * Read off the calendar rather than the queue - a shift moves what already
 * has a date. A move whose release is no longer booked is dropped, as a stale
 * placement is: the backend refuses the shift whole, and a nameless ghost
 * answers nothing in the meantime.
 */
export function shiftGhostsOf(
  moves: readonly Move[],
  slots: readonly ScheduledRelease[],
): Map<string, Ghost[]> {
  return ghostsOf(
    moves.map((move) => ({ release_id: move.release_id, date: move.to })),
    slots,
  )
}
