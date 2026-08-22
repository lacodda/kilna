import { describe, expect, it } from 'vitest'
import type { Readiness } from './api'
import { daysBetween, missing, urgency } from './readiness'

const judged = (overrides: Partial<Readiness>): Readiness => ({
  roles: [],
  scored: true,
  ready: false,
  ...overrides,
})

describe('urgency', () => {
  it('is calm with no date at all — the queue has no deadline', () => {
    expect(urgency(null)).toBe('calm')
  })

  it('turns at the documented bands, on the boundary days themselves', () => {
    expect(urgency(30)).toBe('calm')
    expect(urgency(8)).toBe('calm')
    expect(urgency(7)).toBe('soon')
    expect(urgency(3)).toBe('soon')
    expect(urgency(2)).toBe('urgent')
    expect(urgency(0)).toBe('urgent')
  })

  it('treats an overdue date as urgent, not as calm again', () => {
    expect(urgency(-1)).toBe('urgent')
  })
})

describe('daysBetween', () => {
  it('counts whole days between local ISO dates', () => {
    expect(daysBetween('2026-08-22', '2026-08-29')).toBe(7)
    expect(daysBetween('2026-08-22', '2026-08-22')).toBe(0)
    expect(daysBetween('2026-08-22', '2026-08-20')).toBe(-2)
  })

  it('crosses month and year edges by the calendar, not by string maths', () => {
    expect(daysBetween('2026-08-31', '2026-09-01')).toBe(1)
    expect(daysBetween('2026-12-31', '2027-01-01')).toBe(1)
    // A leap year: 2028-02 has 29 days.
    expect(daysBetween('2028-02-28', '2028-03-01')).toBe(2)
  })
})

describe('missing', () => {
  it('lists the missing roles and skips the inapplicable ones', () => {
    const gaps = missing(
      judged({
        roles: [
          { role: 'lyrics', present: true },
          { role: 'style', present: false },
          { role: 'notes', present: null },
        ],
      }),
    )
    expect(gaps).toEqual(['style'])
  })

  it('adds the score last when nothing speaks for the work', () => {
    expect(missing(judged({ scored: false }))).toEqual(['score'])
    expect(
      missing(judged({ roles: [{ role: 'lyrics', present: false }], scored: false })),
    ).toEqual(['lyrics', 'score'])
  })

  it('is empty exactly when there is nothing to chase', () => {
    expect(missing(judged({ roles: [{ role: 'lyrics', present: true }] }))).toEqual([])
  })
})
