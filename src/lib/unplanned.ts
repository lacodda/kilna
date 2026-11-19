import type { ProfileConfig, ScheduledRelease, ScoredWork, WorkPlan } from '@/lib/api/types'
import { hasDoors, publicationKinds } from '@/lib/useProfile'

/**
 * The works that could go out and have nothing planned yet (v0.96): the
 * catalogue as the calendar's pane lists it, beside the queue.
 *
 * A work goes out once (ADR 0051), so one that already has a release - out,
 * booked or waiting in the queue - has its place already and is not listed
 * again: the queue is where a waiting release is found.
 *
 * A song goes out as what is made from it - its audio, its clip - and never
 * as itself (ADR 0047), so until v0.150.1 it was left out here, and a space
 * whose every publication was already in the queue showed no works at all
 * while a hundred songs waited. It is listed now until something whole made
 * from it - not a short, which never opens a song (ADR 0067) - is out, booked
 * or waiting, and carried onto a day it is planned there (v0.138), its
 * publication made when the day comes. A kind that is a lab is not listed:
 * an experiment goes out as nothing, only what is kept from it does.
 *
 * A work already planned on a day stands on the calendar as that plan.
 *
 * Strongest first, as the queue is ordered, the title breaking ties.
 */
export function unplannedWorks(
  works: readonly ScoredWork[],
  queue: readonly ScheduledRelease[],
  plans: readonly WorkPlan[],
  config: ProfileConfig,
): ScoredWork[] {
  const waiting = new Set(queue.map((entry) => entry.work_id))
  const planned = new Set(plans.map((plan) => plan.work_id))
  const byId = new Map(works.map((work) => [work.work_id, work]))
  const anythingGoesOut = publicationKinds(config).length > 0
  const placed = (work: ScoredWork) =>
    work.released > 0 || work.scheduled > 0 || waiting.has(work.work_id)
  const whole = (kind: string) =>
    config.work_kinds.find((known) => known.key === kind)?.excerpt !== true

  return works
    .filter((work) => {
      if (planned.has(work.work_id)) return false
      if (hasDoors(config, work.kind)) return !placed(work)
      const kind = config.work_kinds.find((known) => known.key === work.kind)
      if (kind === undefined || kind.lab != null || !anythingGoesOut) return false
      return !work.publications.some((id) => {
        const made = byId.get(id)
        return made !== undefined && whole(made.kind) && placed(made)
      })
    })
    .sort(
      (a, b) =>
        Number(a.total === null) - Number(b.total === null) ||
        (b.total ?? 0) - (a.total ?? 0) ||
        a.title.localeCompare(b.title),
    )
}
