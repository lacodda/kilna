import type { ScheduledRelease, TimeOff } from '@/lib/api/types'
import { daysBetween } from '@/lib/readiness'

/**
 * The calendar's closed days, read for drawing (v0.96).
 *
 * A span closes both of its ends and every day between. Spans may overlap -
 * "New Year" inside a fortnight away - and a day they share is named by the
 * one that starts first, as the backend names it under a carried release:
 * the spans arrive in that order.
 */

/** Each closed day of `days`, with the span that closes it. */
export function closedDays(
  spans: readonly TimeOff[],
  days: readonly string[],
): Map<string, TimeOff> {
  const closed = new Map<string, TimeOff>()
  for (const day of days) {
    const span = spans.find((entry) => entry.starts_on <= day && day <= entry.ends_on)
    if (span !== undefined) closed.set(day, span)
  }
  return closed
}

/** How many days a span closes, both ends counted. */
export function spanLength(span: Pick<TimeOff, 'starts_on' | 'ends_on'>): number {
  return daysBetween(span.starts_on, span.ends_on) + 1
}

/**
 * The planned releases a span would close on: what a person may want to move
 * once the days are closed. Pinned ones are left out - their date was settled
 * by hand, and a shift keeps them where they are anyway.
 */
export function bookedInside(
  span: Pick<TimeOff, 'starts_on' | 'ends_on'>,
  slots: readonly ScheduledRelease[],
): ScheduledRelease[] {
  return slots.filter(
    (slot) =>
      slot.status === 'planned' &&
      slot.slot_pinned_at === null &&
      slot.scheduled_at !== null &&
      span.starts_on <= slot.scheduled_at &&
      slot.scheduled_at <= span.ends_on,
  )
}

/** The span as a person reads it: one day, or its two ends. */
export function spanText(
  span: Pick<TimeOff, 'starts_on' | 'ends_on'>,
  day: (date: string) => string,
): string {
  return span.starts_on === span.ends_on
    ? day(span.starts_on)
    : `${day(span.starts_on)} – ${day(span.ends_on)}`
}
