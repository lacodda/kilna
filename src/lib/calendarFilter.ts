import type { ScheduledRelease } from '@/lib/api'
import type { Ghost } from '@/lib/layout'

/**
 * Which release kinds the calendar is showing.
 *
 * `null` is "all of them", not an empty selection: the two look the same in a
 * grid but mean opposite things when the set of kinds changes under them. A
 * profile switch, or a kind the owner deletes, leaves a stored filter naming
 * something that no longer exists -- so the filter names the kinds it keeps and
 * anything unrecognised is simply not matched.
 */
export type KindFilter = string | null

/** The slots a filter lets through, in the order they arrived. */
export function filterByKind(
  slots: readonly ScheduledRelease[],
  kind: KindFilter,
): readonly ScheduledRelease[] {
  if (kind === null) return slots
  return slots.filter((slot) => slot.kind === kind)
}

/**
 * How many slots each kind holds, for the count beside a filter's name.
 *
 * Counted over everything rather than over what is currently shown: a filter
 * that reported "0" for every kind but the active one would be a filter that
 * can only be turned off, never switched.
 */
export function countByKind(slots: readonly ScheduledRelease[]): Map<string, number> {
  const counts = new Map<string, number>()
  for (const slot of slots) counts.set(slot.kind, (counts.get(slot.kind) ?? 0) + 1)
  return counts
}

/**
 * The same choice, applied to the auto-layout ghosts.
 *
 * Without this a filtered month draws a plan it is not showing: pick "clips"
 * and the dashed chips for shorts stay on the grid, so the preview contradicts
 * the very screen it is asking to be approved on. The plan itself is not
 * narrowed -- booking still books every placement, because the filter is a view
 * of the month and not an instruction about what to schedule.
 */
export function filterGhosts(
  ghosts: Map<string, Ghost[]>,
  kind: KindFilter,
): Map<string, Ghost[]> {
  if (kind === null) return ghosts

  const kept = new Map<string, Ghost[]>()
  for (const [date, day] of ghosts) {
    const matching = day.filter((ghost) => ghost.kind === kind)
    // A day left with nothing keeps no entry: the grid asks the map for a day
    // and an empty array would render an empty stack where there is nothing.
    if (matching.length > 0) kept.set(date, matching)
  }
  return kept
}
