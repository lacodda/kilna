import type { ProfileConfig, ScheduledRelease } from '@/lib/api/types'
import { resolveLabel } from '@/lib/label'
import { vocabularyOf } from '@/lib/useProfile'

/**
 * The calendar written out as files (v0.104): a month as a Markdown table,
 * for the plans people keep in their notes, and every dated release as an
 * iCalendar file, for a phone's calendar with nothing else installed.
 *
 * Written here rather than in the core: the words are the window's - the
 * column heads in its language, a door by the name the profile gives it in
 * that language - and the file is saved where the person picks.
 */

/** The words a month's table is headed and filled with. */
export interface TableWords {
  date: string
  time: string
  work: string
  kind: string
  door: string
  status: string
  link: string
  /** A release's status, as the window says it. */
  statusOf: (status: string) => string
}

/** The releases dated in `month` (`YYYY-MM`), as a Markdown table in day
    order, time within a day. A month with nothing dated is the header alone. */
export function monthMarkdown(
  slots: readonly ScheduledRelease[],
  month: string,
  config: ProfileConfig,
  words: TableWords,
  language: string,
): string {
  const rows = slots
    .filter((slot) => slot.scheduled_at?.startsWith(`${month}-`) === true)
    .sort(
      (a, b) =>
        (a.scheduled_at ?? '').localeCompare(b.scheduled_at ?? '') ||
        (a.scheduled_time ?? '').localeCompare(b.scheduled_time ?? ''),
    )
  const head = [
    words.date,
    words.time,
    words.work,
    words.kind,
    words.door,
    words.status,
    words.link,
  ]
  const lines = [row(head), row(head.map(() => '---'))]
  for (const slot of rows) {
    lines.push(
      row([
        slot.scheduled_at ?? '',
        slot.scheduled_time ?? '',
        slot.work_title,
        kindLabel(config, slot.work_kind, language),
        doorLabel(config, slot, language),
        words.statusOf(slot.status),
        slot.url ?? '',
      ]),
    )
  }
  return `${lines.join('\n')}\n`
}

/** One row of a Markdown table: a pipe in a cell would end it, so it is
    escaped, and a line break would end the row, so it is a space. */
function row(cells: readonly string[]): string {
  return `| ${cells.map((cell) => cell.replace(/\|/g, '\\|').replace(/\s*\n\s*/g, ' ')).join(' | ')} |`
}

/**
 * Every dated release as an iCalendar file (RFC 5545): a day-long event for a
 * release with no time, a timed one in its own zone for one with a time.
 * `stamp` is the moment the file is written, as an ISO instant.
 *
 * A release keeps its id as the event's, so the same file imported again
 * updates the events rather than doubling them.
 */
export function calendarIcs(
  slots: readonly ScheduledRelease[],
  config: ProfileConfig,
  language: string,
  stamp: string,
): string {
  const lines = [
    'BEGIN:VCALENDAR',
    'VERSION:2.0',
    'PRODID:-//lacodda//kilna//EN',
    'CALSCALE:GREGORIAN',
    'X-WR-CALNAME:kilna',
  ]
  const now = compact(stamp.slice(0, 19)) + 'Z'
  for (const slot of slots) {
    if (slot.scheduled_at === null) continue
    const day = compact(slot.scheduled_at)
    lines.push('BEGIN:VEVENT', `UID:${slot.id}@kilna`, `DTSTAMP:${now}`)
    if (slot.scheduled_time !== null) {
      const at = `${day}T${compact(slot.scheduled_time)}00`
      lines.push(slot.time_zone === null ? `DTSTART:${at}` : `DTSTART;TZID=${slot.time_zone}:${at}`)
    } else {
      lines.push(
        `DTSTART;VALUE=DATE:${day}`,
        `DTEND;VALUE=DATE:${compact(nextDay(slot.scheduled_at))}`,
      )
    }
    lines.push(`SUMMARY:${text(`${slot.work_title} — ${doorLabel(config, slot, language)}`)}`)
    if (slot.url !== null && slot.url !== '') lines.push(`URL:${slot.url}`)
    lines.push(slot.status === 'released' ? 'STATUS:CONFIRMED' : 'STATUS:TENTATIVE', 'END:VEVENT')
  }
  lines.push('END:VCALENDAR')
  return lines.map(fold).join('\r\n') + '\r\n'
}

/** `2026-11-03` as `20261103`, `12:30` as `1230`. */
function compact(value: string): string {
  return value.replace(/[-:]/g, '')
}

function nextDay(day: string): string {
  const [year, month, date] = day.split('-').map(Number) as [number, number, number]
  return new Date(Date.UTC(year, month - 1, date + 1)).toISOString().slice(0, 10)
}

/** A text value as iCalendar escapes it. */
function text(value: string): string {
  return value
    .replace(/\\/g, '\\\\')
    .replace(/;/g, '\\;')
    .replace(/,/g, '\\,')
    .replace(/\r?\n/g, '\\n')
}

/** A line longer than 75 octets, folded as RFC 5545 says: a line break and
    a space, never inside a character. */
function fold(line: string): string {
  const bytes = new TextEncoder()
  const parts: string[] = []
  let current = ''
  for (const char of line) {
    const limit = parts.length === 0 ? 75 : 74
    if (bytes.encode(current + char).length > limit) {
      parts.push(current)
      current = char
    } else {
      current += char
    }
  }
  parts.push(current)
  return parts.join('\r\n ')
}

function kindLabel(config: ProfileConfig, kind: string, language: string): string {
  const found = config.work_kinds.find((entry) => entry.key === kind)
  return found === undefined ? kind : resolveLabel(found.label, language)
}

function doorLabel(config: ProfileConfig, slot: ScheduledRelease, language: string): string {
  const door = vocabularyOf(config, slot.work_kind).release_kinds.find(
    (entry) => entry.key === slot.kind,
  )
  return door === undefined ? slot.kind : resolveLabel(door.label, language)
}
