import { describe, expect, it } from 'vitest'
import { byDate, daysInMonth, monthGrid, monthOf, shiftMonth, today } from './month'

describe('shiftMonth', () => {
  it('walks forward and back', () => {
    expect(shiftMonth({ year: 2026, month: 5 }, 1)).toEqual({ year: 2026, month: 6 })
    expect(shiftMonth({ year: 2026, month: 5 }, -1)).toEqual({ year: 2026, month: 4 })
  })

  it('rolls over the year in both directions', () => {
    expect(shiftMonth({ year: 2026, month: 12 }, 1)).toEqual({ year: 2027, month: 1 })
    expect(shiftMonth({ year: 2026, month: 1 }, -1)).toEqual({ year: 2025, month: 12 })
  })

  it('walks more than a year at a time', () => {
    expect(shiftMonth({ year: 2026, month: 3 }, 14)).toEqual({ year: 2027, month: 5 })
    expect(shiftMonth({ year: 2026, month: 3 }, -14)).toEqual({ year: 2025, month: 1 })
  })
})

describe('daysInMonth', () => {
  it('knows the short months', () => {
    expect(daysInMonth({ year: 2026, month: 4 })).toBe(30)
    expect(daysInMonth({ year: 2026, month: 1 })).toBe(31)
  })

  it('knows February in an ordinary year and a leap one', () => {
    expect(daysInMonth({ year: 2026, month: 2 })).toBe(28)
    expect(daysInMonth({ year: 2028, month: 2 })).toBe(29)
    // A century that is not a leap year, and one that is.
    expect(daysInMonth({ year: 2100, month: 2 })).toBe(28)
    expect(daysInMonth({ year: 2000, month: 2 })).toBe(29)
  })
})

describe('monthGrid', () => {
  it('is rectangular, always', () => {
    for (let month = 1; month <= 12; month++) {
      expect(monthGrid({ year: 2026, month }).length % 7).toBe(0)
    }
  })

  it('starts on a Monday', () => {
    // 1 September 2026 is a Tuesday, so the grid opens on 31 August.
    const grid = monthGrid({ year: 2026, month: 9 })
    expect(grid[0]).toEqual({ date: '2026-08-31', inMonth: false })
    expect(grid[1]).toEqual({ date: '2026-09-01', inMonth: true })
  })

  it('needs no lead when the month already starts on a Monday', () => {
    // 1 June 2026 is a Monday.
    const grid = monthGrid({ year: 2026, month: 6 })
    expect(grid[0]).toEqual({ date: '2026-06-01', inMonth: true })
  })

  it('carries the whole month between its neighbours', () => {
    const grid = monthGrid({ year: 2026, month: 2 })
    const mine = grid.filter((day) => day.inMonth).map((day) => day.date)
    expect(mine).toHaveLength(28)
    expect(mine[0]).toBe('2026-02-01')
    expect(mine.at(-1)).toBe('2026-02-28')
  })

  it('borrows from the right neighbours across a year boundary', () => {
    const january = monthGrid({ year: 2026, month: 1 })
    // 1 January 2026 is a Thursday: the grid opens in December 2025.
    expect(january[0]?.date.startsWith('2025-12')).toBe(true)

    const december = monthGrid({ year: 2026, month: 12 })
    expect(december.at(-1)?.date.startsWith('2027-01')).toBe(true)
  })

  it('pads the tail with the days that follow, in order', () => {
    const grid = monthGrid({ year: 2026, month: 9 })
    const tail = grid.filter((day) => !day.inMonth && day.date > '2026-09-01')
    expect(tail.map((day) => day.date)).toEqual(['2026-10-01', '2026-10-02', '2026-10-03', '2026-10-04'])
  })
})

describe('today', () => {
  it('reads the local date, not a UTC one', () => {
    // Late evening on the last of the month is where a UTC conversion would
    // roll the date forward for anyone east of Greenwich, and backward for the
    // owner of this app, who is at UTC-3.
    expect(today(new Date(2026, 7, 31, 23, 30))).toBe('2026-08-31')
    expect(today(new Date(2026, 0, 1, 0, 15))).toBe('2026-01-01')
  })
})

describe('monthOf', () => {
  it('reads the month out of an ISO date', () => {
    expect(monthOf('2026-09-04')).toEqual({ year: 2026, month: 9 })
  })
})

describe('byDate', () => {
  it('gathers items onto their day', () => {
    const items = [
      { id: 'a', on: '2026-09-01' },
      { id: 'b', on: '2026-09-01' },
      { id: 'c', on: '2026-09-04' },
    ]
    const grouped = byDate(items, (item) => item.on)
    expect(grouped.get('2026-09-01')?.map((item) => item.id)).toEqual(['a', 'b'])
    expect(grouped.get('2026-09-04')?.map((item) => item.id)).toEqual(['c'])
  })

  it('drops what has no date rather than gathering it under one', () => {
    const items = [{ id: 'a', on: null }, { id: 'b', on: '2026-09-01' }]
    const grouped = byDate(items, (item) => item.on)
    expect(grouped.size).toBe(1)
    expect([...grouped.values()].flat().map((item) => item.id)).toEqual(['b'])
  })
})
