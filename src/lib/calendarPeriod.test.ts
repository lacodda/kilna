import { describe, expect, it } from 'vitest'
import {
  daysOf,
  inPeriod,
  loadView,
  periodOf,
  saveView,
  stepAnchor,
  weekDays,
  weekStart,
} from './calendarPeriod'

function store(initial: Record<string, string> = {}) {
  const data = new Map(Object.entries(initial))
  return {
    getItem: (key: string) => data.get(key) ?? null,
    setItem: (key: string, value: string) => void data.set(key, value),
  }
}

describe('calendar period', () => {
  it('starts a week on Monday, across a month and a year', () => {
    expect(weekStart('2026-10-11')).toBe('2026-10-05') // a Sunday
    expect(weekStart('2026-10-05')).toBe('2026-10-05') // the Monday itself
    expect(weekStart('2027-01-01')).toBe('2026-12-28')
    expect(weekDays('2026-10-01').map((day) => day.date)).toEqual([
      '2026-09-28',
      '2026-09-29',
      '2026-09-30',
      '2026-10-01',
      '2026-10-02',
      '2026-10-03',
      '2026-10-04',
    ])
  })

  it('is the month’s own days for a month and an agenda, never its neighbours', () => {
    expect(periodOf('month', '2026-09-17')).toEqual({ from: '2026-09-01', to: '2026-09-30' })
    expect(periodOf('agenda', '2026-02-10')).toEqual({ from: '2026-02-01', to: '2026-02-28' })
    expect(periodOf('week', '2026-10-11')).toEqual({ from: '2026-10-05', to: '2026-10-11' })
  })

  it('draws whole weeks for the month and only the month for the agenda', () => {
    const month = daysOf('month', '2026-09-17')
    expect(month.length % 7).toBe(0)
    expect(month.some((day) => !day.inMonth)).toBe(true)
    const agenda = daysOf('agenda', '2026-09-17')
    expect(agenda).toHaveLength(30)
    expect(agenda.every((day) => day.inMonth)).toBe(true)
  })

  it('steps a week in the week view and a month otherwise, clamping the day', () => {
    expect(stepAnchor('week', '2026-10-11', 1)).toBe('2026-10-18')
    expect(stepAnchor('week', '2026-10-01', -1)).toBe('2026-09-24')
    expect(stepAnchor('month', '2026-03-31', -1)).toBe('2026-02-28')
    expect(stepAnchor('agenda', '2026-12-15', 1)).toBe('2027-01-15')
  })

  it('holds both ends inside a period', () => {
    const period = { from: '2026-10-05', to: '2026-10-11' }
    expect(inPeriod('2026-10-05', period)).toBe(true)
    expect(inPeriod('2026-10-11', period)).toBe(true)
    expect(inPeriod('2026-10-12', period)).toBe(false)
  })

  it('remembers the view and falls back on anything it does not know', () => {
    const remembered = store()
    expect(loadView(remembered)).toBe('month')
    saveView('agenda', remembered)
    expect(loadView(remembered)).toBe('agenda')
    expect(loadView(store({ 'kilna.calendar.view': 'year' }))).toBe('month')
  })
})
