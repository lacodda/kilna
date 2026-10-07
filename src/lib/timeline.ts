import type { Moment } from '@/lib/api/types'

/**
 * A work's history on one axis, as the History tab draws it (ADR 0056): the
 * backend says what happened and when; this says which day each moment falls
 * on where the person is, which of them are still ahead, and which the
 * filters let through.
 */

/** What the filters of the axis switch on and off. */
export const MOMENT_KINDS = ['versions', 'scores', 'releases', 'journal'] as const
export type MomentKind = (typeof MOMENT_KINDS)[number]

/**
 * Which filter a moment answers to. The beginning answers to none: it is
 * where the axis starts, and an axis that starts nowhere when a filter is
 * off reads as a work with no beginning.
 */
export function kindOf(moment: Moment): MomentKind | null {
  switch (moment.type) {
    case 'begun':
      return null
    case 'version':
      return 'versions'
    case 'score':
      return 'scores'
    case 'made':
    case 'release':
      return 'releases'
    case 'entry':
      return 'journal'
  }
}

const BARE_DAY = /^\d{4}-\d{2}-\d{2}$/

const pad = (value: number) => String(value).padStart(2, '0')

/** The calendar day of a date or a timestamp where the window is, as
 *  `YYYY-MM-DD`. A bare day is that day, whatever the zone. */
export function localDay(at: string): string {
  if (BARE_DAY.test(at)) return at
  const moment = new Date(at)
  if (Number.isNaN(moment.getTime())) return at.slice(0, 10)
  return `${moment.getFullYear()}-${pad(moment.getMonth() + 1)}-${pad(moment.getDate())}`
}

/** Whether a moment says a time of day, or only a day - a release booked
 *  for a date. */
export function hasTime(at: string): boolean {
  return !BARE_DAY.test(at)
}

/** Where a moment stands on the axis: a bare day at its local noon, so no
 *  zone moves it onto the day before. */
function instantOf(at: string): number {
  const read = new Date(BARE_DAY.test(at) ? `${at}T12:00:00` : at).getTime()
  return Number.isNaN(read) ? 0 : read
}

/** Today's and yesterday's days where the window is, for the headings that
 *  name them rather than date them. */
export function nearDays(now: Date = new Date()): { today: string; yesterday: string } {
  const day = (offset: number) => {
    const at = new Date(now.getFullYear(), now.getMonth(), now.getDate() - offset)
    return `${at.getFullYear()}-${pad(at.getMonth() + 1)}-${pad(at.getDate())}`
  }
  return { today: day(0), yesterday: day(1) }
}

export interface Day {
  /** `YYYY-MM-DD`, where the window is. */
  day: string
  moments: Moment[]
}

export interface Axis {
  /** What is booked for a day after today, soonest last - nearest the line
   *  of today, the way the rest of the axis runs. */
  ahead: Moment[]
  /** Today and every day before it that holds a moment, latest first. */
  days: Day[]
}

/**
 * The moments the filters let through, laid out on days where the window is,
 * latest first. The backend sorts too, but by the strings it holds - a
 * timestamp in UTC beside a bare day - and only the window knows which day
 * an evening in another zone falls on.
 */
export function axisOf(
  moments: readonly Moment[],
  shown: ReadonlySet<MomentKind>,
  now: Date = new Date(),
): Axis {
  const { today } = nearDays(now)
  const kept = moments
    .filter((moment) => {
      const kind = kindOf(moment)
      return kind === null || shown.has(kind)
    })
    .map((moment, order) => ({ moment, order, at: instantOf(moment.at) }))
    .sort((a, b) => b.at - a.at || a.order - b.order)

  const ahead: Moment[] = []
  const days: Day[] = []
  for (const { moment } of kept) {
    const day = localDay(moment.at)
    if (day > today) {
      ahead.push(moment)
      continue
    }
    const last = days.at(-1)
    if (last?.day === day) last.moments.push(moment)
    else days.push({ day, moments: [moment] })
  }
  return { ahead, days }
}
