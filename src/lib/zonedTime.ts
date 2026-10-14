/*
 * A release's hour is said in a zone (v0.90): the platform is told 11:00 in
 * Asuncion, and a person in another zone - or the same person travelling -
 * needs to read that as their own clock says it. This is the arithmetic, on
 * the runtime's own zone data (`Intl`), with no library and no table of
 * offsets to go stale.
 */

/** A day and an hour, as the release form writes them: `yyyy-mm-dd`, `HH:MM`. */
export interface DayAndTime {
  date: string
  time: string
}

/**
 * The zones a release can be said in: every IANA name the runtime knows,
 * with `UTC` among them - the backend takes `UTC` and any name with a slash,
 * and the runtime's list leaves `UTC` out on some engines.
 */
export function zoneNames(): string[] {
  const names = known()
  return names.includes('UTC') ? names : ['UTC', ...names]
}

/** The runtime's list, or none from a runtime too old to give one. */
function known(): string[] {
  try {
    return Intl.supportedValuesOf('timeZone')
  } catch {
    return []
  }
}

/** A zone as a person reads it: the underscores of a name turned back into
 *  spaces, `America/Buenos_Aires` → `America/Buenos Aires`. */
export function zoneLabel(zone: string): string {
  return zone.replace(/_/g, ' ')
}

/**
 * The same moment in another zone: `time` on `date` as a clock in `from`
 * shows it, read on a clock in `to`. The day can change - 23:30 in Moscow is
 * the evening before in Asuncion - so both come back.
 *
 * An hour a clock skips (the spring change) is read as the hour after it, as
 * a clock that jumped would show; an hour it shows twice, as the first.
 */
export function inZone(at: DayAndTime, from: string, to: string): DayAndTime {
  const [year, month, day] = at.date.split('-').map(Number)
  const [hour, minute] = at.time.split(':').map(Number)
  const wall = Date.UTC(year ?? 1970, (month ?? 1) - 1, day ?? 1, hour ?? 0, minute ?? 0)
  // The instant whose wall clock in `from` reads `wall`: start from the wall
  // time taken as UTC, correct by the zone's offset there, and correct once
  // more in case the offset changed between the two guesses.
  let instant = wall - offset(wall, from)
  instant = wall - offset(instant, from)
  return wallClock(instant, to)
}

/** How far ahead of UTC a zone's clock is at an instant, in milliseconds. */
function offset(instant: number, zone: string): number {
  const shown = wallClock(instant, zone)
  const [year, month, day] = shown.date.split('-').map(Number)
  const [hour, minute] = shown.time.split(':').map(Number)
  const asUtc = Date.UTC(year ?? 1970, (month ?? 1) - 1, day ?? 1, hour ?? 0, minute ?? 0)
  // Seconds are not part of a wall clock here; the instant's own seconds
  // would otherwise count as offset.
  const truncated = instant - (((instant % 60_000) + 60_000) % 60_000)
  return asUtc - truncated
}

/** What a clock in `zone` shows at an instant. */
function wallClock(instant: number, zone: string): DayAndTime {
  const parts = new Intl.DateTimeFormat('en-CA', {
    timeZone: zone,
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    hourCycle: 'h23',
  }).formatToParts(new Date(instant))
  const part = (type: Intl.DateTimeFormatPartTypes) =>
    parts.find((entry) => entry.type === type)?.value ?? '00'
  return {
    date: `${part('year')}-${part('month')}-${part('day')}`,
    time: `${part('hour')}:${part('minute')}`,
  }
}
