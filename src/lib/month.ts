/**
 * The month grid, as data.
 *
 * Dates here are ISO strings (`yyyy-mm-dd`) throughout, the same shape the
 * database stores and the date picker emits. They are never turned into a
 * `Date` and back for display: `new Date('2026-09-01')` is parsed as UTC and
 * comes back a day earlier for anyone west of Greenwich, which is exactly the
 * kind of bug that only shows up on someone else's machine.
 */

/** A month, identified the way it is addressed: year and 1-based month. */
export interface Month {
  year: number
  month: number
}

export interface Day {
  /** ISO date. */
  date: string
  /** Whether it belongs to the month being drawn, rather than a neighbour. */
  inMonth: boolean
}

export function iso(year: number, month: number, day: number): string {
  return `${year}-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')}`
}

/** The month a date falls in. */
export function monthOf(date: string): Month {
  const [year, month] = date.split('-').map(Number)
  return { year: year ?? 1970, month: month ?? 1 }
}

/** Step a month forward or back, rolling the year over. */
export function shiftMonth({ year, month }: Month, by: number): Month {
  // `month` is 1-based, so shift into 0-based, add, and come back.
  const total = year * 12 + (month - 1) + by
  return { year: Math.floor(total / 12), month: (total % 12) + 1 }
}

export function sameMonth(a: Month, b: Month): boolean {
  return a.year === b.year && a.month === b.month
}

/** How many days a month has, leap years included. */
export function daysInMonth({ year, month }: Month): number {
  // Day 0 of the next month is the last day of this one.
  return new Date(year, month, 0).getDate()
}

/**
 * Every cell of the grid for a month, including the neighbours that fill out
 * the first and last weeks.
 *
 * Weeks start on Monday. That is not a preference: the two calendar screens
 * this replaces both showed a working week, and a release plan is read in
 * working weeks.
 */
export function monthGrid(target: Month): Day[] {
  const first = new Date(target.year, target.month - 1, 1)
  // `getDay()` is 0 for Sunday; shift so Monday is 0.
  const lead = (first.getDay() + 6) % 7

  const days: Day[] = []

  const previous = shiftMonth(target, -1)
  const previousLength = daysInMonth(previous)
  for (let i = lead; i > 0; i--) {
    days.push({ date: iso(previous.year, previous.month, previousLength - i + 1), inMonth: false })
  }

  const length = daysInMonth(target)
  for (let day = 1; day <= length; day++) {
    days.push({ date: iso(target.year, target.month, day), inMonth: true })
  }

  // Fill the last week out so the grid is rectangular; a ragged final row makes
  // the whole month look broken rather than finished.
  const next = shiftMonth(target, 1)
  let day = 1
  while (days.length % 7 !== 0) {
    days.push({ date: iso(next.year, next.month, day), inMonth: false })
    day += 1
  }

  return days
}

/** Today, as an ISO date in the local timezone. */
export function today(now: Date = new Date()): string {
  return iso(now.getFullYear(), now.getMonth() + 1, now.getDate())
}

/** Group things carrying a date by that date. */
export function byDate<T>(items: readonly T[], dateOf: (item: T) => string | null): Map<string, T[]> {
  const grouped = new Map<string, T[]>()

  for (const item of items) {
    const date = dateOf(item)
    if (date === null) continue

    const slot = grouped.get(date)
    if (slot === undefined) grouped.set(date, [item])
    else slot.push(item)
  }

  return grouped
}
