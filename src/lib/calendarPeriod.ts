import { addDays, addMonths, weekday } from '@/components/ui/calendar-math'
import { monthGrid, monthOf, type Day } from '@/lib/month'
import type { LayoutStore } from '@/lib/calendarLayout'

/**
 * How the calendar is looked at, and which stretch of days that shows.
 *
 * Three ways of reading one plan (v0.96). The month answers "is that week
 * empty"; the week is the same days at four times the size, where a busy
 * Friday shows everything it holds and each chip has room for its cover; the
 * agenda is the month as a list, every release in full and the empty days
 * folded into one line that says how many - the gaps a grid makes you count.
 *
 * The screen holds one day, the anchor, and each view shows the stretch
 * around it: its month, or its week. Moving between views keeps the anchor,
 * so the week opened from a month is a week of that month.
 */
export type CalendarView = 'month' | 'week' | 'agenda'

export const VIEWS: readonly CalendarView[] = ['month', 'week', 'agenda']

const DEFAULT_VIEW: CalendarView = 'month'

/** A stretch of days, both ends inside it. */
export interface Period {
  from: string
  to: string
}

/** The Monday of the week `date` falls in. Weeks start on Monday here, as
 *  the month's do (`monthGrid`): a release plan is read in working weeks. */
export function weekStart(date: string): string {
  return addDays(date, 1 - weekday(date))
}

/** The seven days of the week `date` falls in, Monday first. */
export function weekDays(date: string): Day[] {
  const monday = weekStart(date)
  return Array.from({ length: 7 }, (_, index) => ({
    date: addDays(monday, index),
    inMonth: true,
  }))
}

/** The days a view draws around `anchor`: the month's whole weeks, the
 *  week, or the month's own days for the agenda. */
export function daysOf(view: CalendarView, anchor: string): Day[] {
  if (view === 'week') return weekDays(anchor)
  const grid = monthGrid(monthOf(anchor))
  return view === 'agenda' ? grid.filter((day) => day.inMonth) : grid
}

/**
 * The stretch a view is about: what "this month" or "this week" means to an
 * action on it - the fields written in one pass, the dates kept in one
 * gesture. The month's own days, never the neighbours that fill its weeks:
 * pinning "September" must not pin the 1st of October.
 */
export function periodOf(view: CalendarView, anchor: string): Period {
  if (view === 'week') {
    const from = weekStart(anchor)
    return { from, to: addDays(from, 6) }
  }
  const days = monthGrid(monthOf(anchor)).filter((day) => day.inMonth)
  return { from: days[0]?.date ?? anchor, to: days[days.length - 1]?.date ?? anchor }
}

/** The anchor one step on: a week in the week, a month otherwise. */
export function stepAnchor(view: CalendarView, anchor: string, by: number): string {
  return view === 'week' ? addDays(anchor, 7 * by) : addMonths(anchor, by)
}

/** Whether `date` is inside `period`. */
export function inPeriod(date: string, period: Period): boolean {
  return period.from <= date && date <= period.to
}

const VIEW_KEY = 'kilna.calendar.view'

function isView(value: unknown): value is CalendarView {
  return value === 'month' || value === 'week' || value === 'agenda'
}

/** The view survives a restart: which way a person reads the plan is a
 *  standing choice, like how much of the screen the month gets. */
export function loadView(store: LayoutStore = localStorage): CalendarView {
  try {
    const raw = store.getItem(VIEW_KEY)
    return isView(raw) ? raw : DEFAULT_VIEW
  } catch {
    return DEFAULT_VIEW
  }
}

export function saveView(view: CalendarView, store: LayoutStore = localStorage): void {
  try {
    store.setItem(VIEW_KEY, view)
  } catch {
    // Storage full or blocked: the view still changes, it just forgets.
  }
}
