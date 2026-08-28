import type { ScheduledRelease, ScoredWork } from '@/lib/api'
import { daysBetween } from '@/lib/readiness'

/**
 * What the first screen shows, worked out from the catalogue and the calendar.
 *
 * No new backend command: everything here is a reading of the two queries the
 * catalogue and the calendar already make. That is deliberate — a dashboard
 * with a query of its own is a second place for "what is ready" to be decided,
 * and the two would drift.
 *
 * The rule inherited from the catalogue's gaps, and from the predecessor's
 * mistake that produced it: **only what can still be acted on**. A dashboard
 * that lists finished work offering to fix it is worse than an empty one.
 */

/** How far ahead "this week" reaches. Matches the journal's warning horizon. */
export const WEEK_AHEAD_DAYS = 7

/** How many works the "what to work on" grid shows before it stops being a shortlist. */
export const SHORTLIST_LIMIT = 6

/** A release that needs something before its date arrives. */
export interface Decision {
  release: ScheduledRelease
  /** Whole days until the slot; negative when the date has passed. */
  daysLeft: number
}

export interface Dashboard {
  /** Unready releases whose date is close, soonest first. */
  decisions: Decision[]
  /** Everything with a slot inside the week, soonest first. */
  week: Decision[]
  /** Scored work that has gone nowhere yet, strongest first. */
  shortlist: ScoredWork[]
  /** Works nothing has judged, so nothing can rank them. */
  unscored: ScoredWork[]
}

/**
 * Read the two lists into the three questions the screen asks.
 *
 * `today` is passed in rather than taken from the clock: the backend only
 * knows UTC, and every date decision in kilna is made against the user's own
 * day — the v0.23 lesson.
 */
export function summarise(
  works: readonly ScoredWork[],
  calendar: readonly ScheduledRelease[],
  today: string,
): Dashboard {
  const dated = calendar
    .filter((entry) => entry.scheduled_at !== null)
    .map((entry) => ({
      release: entry,
      daysLeft: daysBetween(today, entry.scheduled_at as string),
    }))

  // A release that has already gone out needs nothing, whatever its readiness
  // says: the roles it lacked are a fact about the past now.
  const pending = dated.filter(({ release }) => release.released_at === null)

  const week = pending
    .filter(({ daysLeft }) => daysLeft >= 0 && daysLeft <= WEEK_AHEAD_DAYS)
    .sort(bySoonest)

  // Overdue slots are decisions too, and the loudest kind: the date passed and
  // the work still is not ready. They sort ahead of everything by being the
  // most negative.
  const decisions = pending
    .filter(({ release }) => !release.readiness.ready)
    .filter(({ daysLeft }) => daysLeft <= WEEK_AHEAD_DAYS)
    .sort(bySoonest)

  const shortlist = works
    .filter((work) => work.total !== null && work.released === 0 && work.scheduled === 0)
    .sort((a, b) => (b.total ?? 0) - (a.total ?? 0) || a.title.localeCompare(b.title))
    .slice(0, SHORTLIST_LIMIT)

  const unscored = works
    .filter((work) => work.total === null && work.released === 0)
    .sort((a, b) => a.title.localeCompare(b.title))

  return { decisions, week, shortlist, unscored }
}

/** Soonest first, and a passed date before an approaching one. */
function bySoonest(a: Decision, b: Decision): number {
  return a.daysLeft - b.daysLeft || a.release.work_title.localeCompare(b.release.work_title)
}

/**
 * Whether the whole screen has nothing to say.
 *
 * Not the same as an empty workspace: a person whose work is all scored,
 * scheduled and shipped sees this too, and that is the screen doing its job
 * rather than failing to.
 */
export function isQuiet(dashboard: Dashboard): boolean {
  return (
    dashboard.decisions.length === 0 &&
    dashboard.week.length === 0 &&
    dashboard.shortlist.length === 0 &&
    dashboard.unscored.length === 0
  )
}
