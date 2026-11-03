import { weekStart } from '@/lib/calendarPeriod'
import { today } from '@/lib/month'

/**
 * The moments this window's day and week began, as the backend counts runs
 * from them (v0.107).
 *
 * The day and the week are the person's: after sunset here UTC is already
 * tomorrow, and a run asked at nine in the evening belongs to today. So the
 * window says where its midnight and its Monday fall, as moments, and the
 * backend sums the runs that ended since.
 */
export type Since = {
  dayFrom: string
  weekFrom: string
}

/** The moment a local date's midnight falls on. */
function midnightOf(date: string): string {
  // A date and time with no zone is read as local time.
  return new Date(`${date}T00:00:00`).toISOString()
}

export function spendSince(now: Date = new Date()): Since {
  const day = today(now)
  return { dayFrom: midnightOf(day), weekFrom: midnightOf(weekStart(day)) }
}
