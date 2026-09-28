import { describe, expect, it } from 'vitest'
import {
  formatCost,
  formatDay,
  formatDelta,
  formatDuration,
  formatMoment,
  formatMonth,
  formatNumber,
  formatStamp,
} from '@/lib/format'

// A fixed "now", so a date's year and a moment's day read the same any day.
const NOW = new Date('2026-09-15T15:00:00')

describe('a number', () => {
  it('keeps its decimals, in the window’s own separator', () => {
    expect(formatNumber(7.5, 1, 'en')).toBe('7.5')
    expect(formatNumber(7.5, 1, 'ru')).toBe('7,5')
    expect(formatNumber(68, 1, 'en')).toBe('68.0')
    expect(formatNumber(67.6, 0, 'en')).toBe('68')
  })

  it('carries its sign when it is a change, and none when there is none', () => {
    expect(formatDelta(2.5, 1, 'en')).toBe('+2.5')
    expect(formatDelta(-1, 1, 'ru')).toBe('-1,0')
    expect(formatDelta(0, 1, 'en')).toBe('0.0')
  })

  it('is a price in dollars where the dollar goes in each language', () => {
    expect(formatCost(0.02, 2, 'en')).toBe('$0.02')
    expect(formatCost(0.02, 2, 'ru')).toMatch(/^0,02\s\$$/)
  })
})

describe('a date', () => {
  it('is a day and a short month this year, with the year before it', () => {
    expect(formatDay('2026-09-01', 'en', NOW)).toBe('Sep 1')
    expect(formatDay('2025-09-01', 'en', NOW)).toBe('Sep 1, 2025')
    expect(formatDay('2026-09-01', 'ru', NOW)).toBe('1 сент.')
  })

  it('reads a timestamp by the day it was written where the window is', () => {
    expect(formatDay('2026-09-01T10:00:00Z', 'en', NOW)).toBe('Sep 1')
  })

  it('names a month and its year', () => {
    expect(formatMonth(2026, 9, 'en')).toBe('September 2026')
    expect(formatMonth(2026, 9, 'ru')).toMatch(/^сентябрь 2026/)
  })

  it('is a time today and a day before it, as a feed says it', () => {
    expect(formatMoment('2026-09-15T10:05:00', 'en', NOW)).toMatch(/^10:05/)
    expect(formatMoment('2026-09-14T10:05:00', 'en', NOW)).toBe('Sep 14')
  })

  it('keeps its time on every day, as a record says it', () => {
    expect(formatStamp('2026-09-15T10:05:00', 'en', NOW)).toMatch(/^today · 10:05/)
    expect(formatStamp('2026-09-14T23:50:00', 'en', NOW)).toMatch(/^yesterday · 11:50/)
    expect(formatStamp('2026-09-13T10:05:00', 'en', NOW)).toMatch(/^Sep 13 · 10:05/)
    expect(formatStamp('2025-09-13T10:05:00', 'en', NOW)).toMatch(/^Sep 13, 2025 · 10:05/)
    expect(formatStamp('2026-09-14T10:05:00', 'ru', NOW)).toBe('вчера · 10:05')
  })

  it('counts days by the calendar, not by twenty-four hours', () => {
    // Ten minutes after midnight, a line from 23:50 is yesterday's.
    const justAfter = new Date('2026-09-15T00:10:00')
    expect(formatStamp('2026-09-14T23:50:00', 'en', justAfter)).toMatch(/^yesterday/)
  })

  it('says a malformed moment as it came', () => {
    expect(formatStamp('not a date', 'en', NOW)).toBe('not a date')
  })
})

describe('a duration', () => {
  it('reads in seconds under a minute', () => {
    expect(formatDuration(4200, 'en')).toBe('4s')
    expect(formatDuration(59_400, 'en')).toBe('59s')
  })

  it('reads in minutes and seconds, zero-padded', () => {
    expect(formatDuration(72_000, 'en')).toBe('1m 12s')
    expect(formatDuration(605_000, 'en')).toBe('10m 05s')
  })

  it('reads in hours past the hour', () => {
    expect(formatDuration(3_900_000, 'en')).toBe('1h 05m')
  })

  it('says less than a second rather than none', () => {
    // A run that did happen must not read as instant.
    expect(formatDuration(400, 'en')).toBe('<1s')
  })

  it('is said in the window’s language', () => {
    expect(formatDuration(605_000, 'ru')).toBe('10 мин 05 с')
  })

  it('has nothing to say about nothing', () => {
    expect(formatDuration(null)).toBeNull()
    expect(formatDuration(Number.NaN)).toBeNull()
    expect(formatDuration(-5)).toBeNull()
  })
})
