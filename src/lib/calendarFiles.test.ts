import { describe, expect, it } from 'vitest'
import type { ProfileConfig, ScheduledRelease } from '@/lib/api/types'
import { calendarIcs, monthMarkdown, type TableWords } from '@/lib/calendarFiles'

const CONFIG = {
  work_kinds: [
    {
      key: 'audio',
      label: { en: 'Audio', ru: 'Аудио' },
      release_kinds: [{ key: 'youtube', label: { en: 'YouTube', ru: 'Ютуб' }, requires: [] }],
    },
  ],
} as unknown as ProfileConfig

function slot(over: Partial<ScheduledRelease>): ScheduledRelease {
  return {
    id: 'r1',
    work_id: 'w1',
    work_title: 'Tide (audio)',
    work_kind: 'audio',
    kind: 'youtube',
    status: 'planned',
    scheduled_at: '2026-11-03',
    released_at: null,
    url: null,
    slot_pinned_at: null,
    scheduled_time: null,
    time_zone: null,
    ...over,
  } as ScheduledRelease
}

const WORDS: TableWords = {
  date: 'Date',
  time: 'Time',
  work: 'Work',
  kind: 'Kind',
  door: 'Door',
  status: 'Status',
  link: 'Link',
  statusOf: (status) => (status === 'released' ? 'out' : 'planned'),
}

describe('a month as Markdown', () => {
  it('lists the month in day order, in the words of the window', () => {
    const text = monthMarkdown(
      [
        slot({ id: 'b', scheduled_at: '2026-11-10', work_title: 'A | B' }),
        slot({ id: 'a', scheduled_time: '12:00' }),
        slot({ id: 'c', scheduled_at: '2026-12-01' }),
      ],
      '2026-11',
      CONFIG,
      WORDS,
      'ru',
    )

    expect(text).toBe(
      [
        '| Date | Time | Work | Kind | Door | Status | Link |',
        '| --- | --- | --- | --- | --- | --- | --- |',
        '| 2026-11-03 | 12:00 | Tide (audio) | Аудио | Ютуб | planned |  |',
        '| 2026-11-10 |  | A \\| B | Аудио | Ютуб | planned |  |',
        '',
      ].join('\n'),
    )
  })
})

describe('the calendar as iCalendar', () => {
  it('writes a day-long event, or a timed one in its zone, with lines ended as the format says', () => {
    const text = calendarIcs(
      [
        slot({}),
        slot({
          id: 'r2',
          scheduled_time: '12:30',
          time_zone: 'Europe/Moscow',
          status: 'released',
          url: 'https://example.com/v',
        }),
        slot({ id: 'r3', scheduled_at: null }),
      ],
      CONFIG,
      'en',
      '2026-10-27T10:00:00.000Z',
    )

    const lines = text.split('\r\n')
    expect(lines[0]).toBe('BEGIN:VCALENDAR')
    expect(lines).toContain('UID:r1@kilna')
    expect(lines).toContain('DTSTART;VALUE=DATE:20261103')
    expect(lines).toContain('DTEND;VALUE=DATE:20261104')
    expect(lines).toContain('DTSTART;TZID=Europe/Moscow:20261103T123000')
    expect(lines).toContain('SUMMARY:Tide (audio) — YouTube')
    expect(lines).toContain('STATUS:CONFIRMED')
    expect(lines).toContain('DTSTAMP:20261027T100000Z')
    expect(lines.filter((line) => line === 'BEGIN:VEVENT')).toHaveLength(2)
    expect(text.endsWith('END:VCALENDAR\r\n')).toBe(true)
  })

  it('escapes what the format reserves and folds a long line', () => {
    const text = calendarIcs(
      [slot({ work_title: `Tide, part; one ${'long '.repeat(20)}` })],
      CONFIG,
      'en',
      '2026-10-27T10:00:00.000Z',
    )
    expect(text).toContain('SUMMARY:Tide\\, part\\; one')
    const lines = text.split('\r\n')
    expect(lines.every((line) => new TextEncoder().encode(line).length <= 75)).toBe(true)
    expect(lines.some((line) => line.startsWith(' '))).toBe(true)
  })
})
