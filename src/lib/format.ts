import i18n from '@/i18n'

/*
 * Numbers, dates and durations, said in the interface's language.
 *
 * One place, because until v0.77 there were thirty: a score was `toFixed(1)`,
 * so a Russian window read "7.5" where it writes "7,5"; a date in a table was
 * the first ten characters of a timestamp, "2026-09-15", while the journal
 * beside it said "Sep 15"; a run took "1m 05s" in every language. Everything
 * here asks `Intl` in the language the window is in - not the machine's - so
 * a Russian interface on an English machine reads as Russian throughout.
 *
 * Each takes the language as an optional last argument for the tests; the
 * window never passes it.
 */

const current = () => i18n.resolvedLanguage ?? 'en'

/** Formatters are not cheap to build and are asked for per cell; kept per shape. */
const cache = new Map<string, Intl.NumberFormat | Intl.DateTimeFormat>()

function numbers(language: string, options: Intl.NumberFormatOptions): Intl.NumberFormat {
  const key = `n|${language}|${JSON.stringify(options)}`
  let found = cache.get(key) as Intl.NumberFormat | undefined
  if (found === undefined) {
    found = new Intl.NumberFormat(language, options)
    cache.set(key, found)
  }
  return found
}

function dates(language: string, options: Intl.DateTimeFormatOptions): Intl.DateTimeFormat {
  const key = `d|${language}|${JSON.stringify(options)}`
  let found = cache.get(key) as Intl.DateTimeFormat | undefined
  if (found === undefined) {
    found = new Intl.DateTimeFormat(language, options)
    cache.set(key, found)
  }
  return found
}

/**
 * A number with a fixed count of decimals: a score's `7.5`, a total's `68`.
 * The count is always shown, so a column of scores lines up.
 */
export function formatNumber(value: number, decimals = 1, language = current()): string {
  return numbers(language, {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  }).format(value)
}

/** A change, signed: `+2.5`, `-1.0`; nothing is said about no change but `0.0`. */
export function formatDelta(value: number, decimals = 1, language = current()): string {
  return numbers(language, {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
    signDisplay: 'exceptZero',
  }).format(value)
}

/** What a run cost, in the dollars the CLI reports it in. */
export function formatCost(usd: number, decimals = 2, language = current()): string {
  return numbers(language, {
    style: 'currency',
    currency: 'USD',
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  }).format(usd)
}

/**
 * The day part of a date or a timestamp, as a string of `YYYY-MM-DD`: a
 * timestamp's own calendar day where it was written, not the UTC one.
 */
function dayOf(value: string): Date {
  // A bare date is a day, not a moment: read at noon so no time zone moves it.
  return /^\d{4}-\d{2}-\d{2}$/.test(value) ? new Date(`${value}T12:00:00`) : new Date(value)
}

/**
 * A day: "Sep 15", "15 сент." - with the year only when it is not this one,
 * where it would be the one thing worth saying.
 */
export function formatDay(value: string, language = current(), now = new Date()): string {
  const day = dayOf(value)
  // Said as it came rather than thrown: a malformed date on one row must not
  // take the screen down with it.
  if (Number.isNaN(day.getTime())) return value
  const thisYear = day.getFullYear() === now.getFullYear()
  return dates(language, {
    day: 'numeric',
    month: 'short',
    ...(thisYear ? {} : { year: 'numeric' }),
  }).format(day)
}

/** A month and its year, as a calendar names the month it shows. */
export function formatMonth(year: number, month: number, language = current()): string {
  return dates(language, { month: 'long', year: 'numeric' }).format(new Date(year, month - 1, 1))
}

/**
 * When something happened, as a feed says it: the time for today, the day
 * for anything before.
 */
export function formatMoment(timestamp: string, language = current(), now = new Date()): string {
  const at = new Date(timestamp)
  if (Number.isNaN(at.getTime())) return timestamp
  const today = at.toDateString() === now.toDateString()
  return today
    ? dates(language, { hour: '2-digit', minute: '2-digit' }).format(at)
    : formatDay(timestamp, language, now)
}

/**
 * How long something took: `4s`, `1m 12s`, `2h 05m` in English, each
 * language's own narrow units elsewhere. Rounded to the second, because a run
 * is answered by a model over a network and the milliseconds are noise; under
 * a second it says `<1s` rather than `0s`, which would read as "instant" for
 * something that did happen. Null for a length nobody measured.
 */
export function formatDuration(ms: number | null, language = current()): string | null {
  if (ms === null || !Number.isFinite(ms) || ms < 0) return null
  const unit = (value: number, name: string, pad = false) =>
    numbers(language, {
      style: 'unit',
      unit: name,
      unitDisplay: 'narrow',
      minimumIntegerDigits: pad ? 2 : 1,
    }).format(value)

  const seconds = Math.round(ms / 1000)
  if (seconds < 1) return `<${unit(1, 'second')}`
  if (seconds < 60) return unit(seconds, 'second')
  const minutes = Math.floor(seconds / 60)
  if (minutes < 60) return `${unit(minutes, 'minute')} ${unit(seconds % 60, 'second', true)}`
  return `${unit(Math.floor(minutes / 60), 'hour')} ${unit(minutes % 60, 'minute', true)}`
}
