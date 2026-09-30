import type { ScheduledRelease, ScoredWork } from '@/lib/api/types'
import { identify, type Finding, type FindingKind } from '@/lib/findings'
import { daysBetween, missing } from '@/lib/readiness'

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
const WEEK_AHEAD_DAYS = 7

/**
 * How many judged, unbooked works are put forward to be scheduled.
 *
 * The strongest few are a decision; forty of them are a list, and the
 * catalogue is where lists are read.
 */
export const SCHEDULE_LIMIT = 6

/** How many of the coming slots "closest to going out" shows: one row of covers, two at most. */
export const NEAREST_LIMIT = 4

/** A release holding a date, and how far off that date is. */
export interface Slot {
  release: ScheduledRelease
  /** Whole days until the slot; negative when the date has passed. */
  daysLeft: number
}

export interface Dashboard {
  /** Unready releases whose date is close, soonest first. */
  decisions: Slot[]
  /** Everything with a slot inside the week, soonest first. */
  week: Slot[]
  /** The next slots still to come, ready or not, soonest first. */
  nearest: Slot[]
  /** Works nothing has judged, so nothing can rank them. */
  unscored: ScoredWork[]
}

/**
 * Read the two lists into the questions the screen asks.
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

  // Slots, not works. Until v0.79 this was the strongest scored work with
  // nothing booked, under a heading that promised what goes out next - and
  // what goes out next is whatever holds the nearest date. A passed date is
  // not going out any more: it is late, and a late one that is not ready is
  // already a decision above.
  const nearest = pending
    .filter(({ daysLeft }) => daysLeft >= 0)
    .sort(bySoonest)
    .slice(0, NEAREST_LIMIT)

  const unscored = works
    .filter((work) => work.total === null && work.released === 0)
    .sort((a, b) => a.title.localeCompare(b.title))

  return { decisions, week, nearest, unscored }
}

/** Soonest first, and a passed date before an approaching one. */
function bySoonest(a: Slot, b: Slot): number {
  return a.daysLeft - b.daysLeft || a.release.work_title.localeCompare(b.release.work_title)
}

/**
 * Whether the lead column has nothing to say.
 *
 * Not the same as an empty workspace: a person whose work is all scored,
 * scheduled and shipped sees this too, and that is the screen doing its job
 * rather than failing to.
 */
export function isQuiet(dashboard: Dashboard): boolean {
  return (
    dashboard.decisions.length === 0 &&
    dashboard.week.length === 0 &&
    dashboard.nearest.length === 0 &&
    dashboard.unscored.length === 0
  )
}

/**
 * The one move a decision's button makes, and so the word on it.
 *
 * Each is a way to the place the thing is done, never the thing done from
 * here: the dashboard changes nothing (since v0.28), and a score, a date or a
 * missing draft are each made on a tab that shows what they are made from.
 */
export type Move = 'score' | 'open' | 'rescore' | 'schedule'

interface DecisionBase {
  /** Stable across renders and unique on the screen. */
  key: string
  workId: string
  title: string
  move: Move
  /** The tab of the work the move is made on. */
  tab: 'score' | 'versions' | 'releases'
}

/**
 * Something only the person can settle, with the move that settles it.
 *
 * Two sources. A release whose date is close and whose work is not ready -
 * its reason is what it lacks. And the two findings that have exactly one
 * answer: a score the draft has outgrown is re-scored, judged work with
 * nothing booked is scheduled. Their reason is the complaint itself, and a
 * dismissal hides them here as it does anywhere.
 */
export type Decision =
  | (DecisionBase & {
      kind: 'release'
      slot: Slot
      /** What the release still lacks: role keys, then `'score'`. */
      gaps: string[]
    })
  | (DecisionBase & {
      kind: 'finding'
      finding: Finding
      /** The work's score, for the sentence that gives the reason. */
      total: number | null
    })

/**
 * The decisions, most pressing first: dated ones by their date, then scores
 * to redo, then the strongest work waiting for a slot.
 *
 * One card per work. A work whose release lacks a cover and whose score went
 * stale has two things wrong and one next step - the date is the one that will
 * not wait - and two cards with the same cover read as the same card drawn
 * twice. The complaint left out is not lost: it is not on a card, so the
 * findings beside the decisions say it (`aside`).
 *
 * `standing` is the findings left after the person's dismissals.
 */
export function decide(
  dashboard: Dashboard,
  standing: readonly Finding[],
  works: readonly ScoredWork[],
): Decision[] {
  const decided: Decision[] = dashboard.decisions.map((slot) => {
    const { release } = slot
    // Straight to the tab that closes the gap: an unscored work to its score,
    // a missing role to the versions. The row this replaced opened the
    // overview for a missing role while its comment promised the versions.
    const scored = release.readiness.scored
    return {
      kind: 'release',
      key: `release:${release.id}`,
      workId: release.work_id,
      title: release.work_title,
      move: scored ? 'open' : 'score',
      tab: scored ? 'versions' : 'score',
      slot,
      gaps: missing(release.readiness),
    }
  })

  const held = new Set(decided.map((decision) => decision.workId))
  const totals = new Map(works.map((work) => [work.work_id, work.total]))
  const card = (finding: Finding, move: 'rescore' | 'schedule'): Decision => ({
    kind: 'finding',
    key: identify(finding),
    workId: finding.workId,
    title: finding.title,
    move,
    tab: move === 'rescore' ? 'score' : 'releases',
    finding,
    total: totals.get(finding.workId) ?? null,
  })

  // Re-scoring comes before scheduling, for the same work too: a date given
  // on a score the draft has outgrown is given on the wrong evidence.
  for (const finding of standing) {
    if (finding.kind !== 'stale-score' || held.has(finding.workId)) continue
    decided.push(card(finding, 'rescore'))
    held.add(finding.workId)
  }

  const waiting = standing
    .filter((finding) => finding.kind === 'ready-unscheduled' && !held.has(finding.workId))
    .sort(
      (a, b) =>
        (totals.get(b.workId) ?? 0) - (totals.get(a.workId) ?? 0) || a.title.localeCompare(b.title),
    )
    .slice(0, SCHEDULE_LIMIT)
  for (const finding of waiting) decided.push(card(finding, 'schedule'))

  return decided
}

/**
 * Kinds the lead column accounts for whole, so the findings beside it leave
 * them out: every unscored work is listed there, and judged work waiting for a
 * slot is put forward strongest first - the ones past the limit are the
 * catalogue's to list, not forty complaints in a column.
 */
const ACCOUNTED_FOR: readonly FindingKind[] = ['unscored', 'ready-unscheduled']

/**
 * The findings that stand beside the decisions: everything not already said
 * on the screen. Said once, somewhere - never twice, never nowhere.
 */
export function aside(standing: readonly Finding[], decisions: readonly Decision[]): Finding[] {
  const carded = new Set(
    decisions.flatMap((decision) => (decision.kind === 'finding' ? [decision.key] : [])),
  )
  return standing.filter(
    (finding) => !ACCOUNTED_FOR.includes(finding.kind) && !carded.has(identify(finding)),
  )
}

/** The catalogue in three figures, for the widget that says how big it is. */
export interface Tally {
  works: number
  /** Something has judged it. */
  scored: number
  /** Nothing has, and it has not gone out - the unscored the lead column lists. */
  unscored: number
  /** At least one of its releases has gone out. */
  released: number
}

export function tally(works: readonly ScoredWork[]): Tally {
  return {
    works: works.length,
    scored: works.filter((work) => work.total !== null).length,
    unscored: works.filter((work) => work.total === null && work.released === 0).length,
    released: works.filter((work) => work.released > 0).length,
  }
}

/** What a running task is doing, and to which work when it is about one. */
export interface TaskSubject {
  action: string
  workId: string | null
}

/**
 * Read a task key back into its action and its work.
 *
 * The key is built by `taskKey` and its siblings in `lib/tasks`: the action,
 * then the work, then a scene and a block when the task is about one. A task
 * about a comment, a screenshot or a release carries a marker where the work
 * would be, and is about no work at all: a release's work is the release's to
 * say, not the key's.
 */
export function subjectOf(key: string): TaskSubject {
  const [action = '', second] = key.split(':')
  if (
    second === undefined ||
    second === '' ||
    second === 'comment' ||
    second === 'channel' ||
    second === 'release'
  ) {
    return { action, workId: null }
  }
  return { action, workId: second }
}
