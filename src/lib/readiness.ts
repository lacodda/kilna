import type { Readiness } from './api'

// The same gap means something different a month out and two days out. The
// bands match the journal's warning horizon: inside a week a gap is worth
// amber, inside two days it is red.
export type Urgency = 'calm' | 'soon' | 'urgent'

const URGENT_WITHIN_DAYS = 2
const SOON_WITHIN_DAYS = 7

/** How loudly a gap should be shown, given how close the date is. */
export function urgency(daysLeft: number | null): Urgency {
  if (daysLeft === null || daysLeft > SOON_WITHIN_DAYS) return 'calm'
  if (daysLeft > URGENT_WITHIN_DAYS) return 'soon'
  return 'urgent'
}

/**
 * Whole days from one local ISO date to another, negative when `to` is past.
 *
 * Both dates go through `Date.UTC` from their parts: `new Date('2026-09-01')`
 * parses as UTC midnight and shifts a day in negative-offset timezones, which
 * is exactly the bug the calendar had in v0.23.
 */
export function daysBetween(from: string, to: string): number {
  return Math.round((utcMidnight(to) - utcMidnight(from)) / 86_400_000)
}

function utcMidnight(date: string): number {
  return Date.UTC(Number(date.slice(0, 4)), Number(date.slice(5, 7)) - 1, Number(date.slice(8, 10)))
}

/**
 * What the release still needs: the role keys marked missing, then `'score'`
 * when nothing speaks for the work. Empty exactly when the release is ready.
 */
export function missing(readiness: Readiness): string[] {
  const roles = readiness.roles
    .filter((mark) => mark.present === false)
    .map((mark) => mark.role)
  return readiness.scored ? roles : [...roles, 'score']
}
