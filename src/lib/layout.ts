import type { Placement, ScheduledRelease } from '@/lib/api'

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
