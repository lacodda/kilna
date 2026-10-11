import type { ProfileConfig, ScheduledRelease, ScoredWork } from '@/lib/api/types'
import { vocabularyOf } from '@/lib/useProfile'

/**
 * The works that could go out and have nothing planned yet (v0.96): the
 * catalogue as the calendar's pane lists it, beside the queue.
 *
 * A work goes out once (ADR 0051), so one that already has a release - out,
 * booked or waiting in the queue - has its place already and is not listed
 * again: the queue is where a waiting release is found. A kind with no doors
 * - a song - never goes out itself, and dropping one on a day could only be
 * refused.
 *
 * Strongest first, as the queue is ordered, the title breaking ties.
 */
export function unplannedWorks(
  works: readonly ScoredWork[],
  queue: readonly ScheduledRelease[],
  config: ProfileConfig,
): ScoredWork[] {
  const waiting = new Set(queue.map((entry) => entry.work_id))
  return works
    .filter(
      (work) =>
        work.released === 0 &&
        work.scheduled === 0 &&
        !waiting.has(work.work_id) &&
        vocabularyOf(config, work.kind).release_kinds.length > 0,
    )
    .sort(
      (a, b) =>
        Number(a.total === null) - Number(b.total === null) ||
        (b.total ?? 0) - (a.total ?? 0) ||
        a.title.localeCompare(b.title),
    )
}
