import { describe, expect, it } from 'vitest'
import { spendSince } from '@/lib/spend'

describe('the span runs are counted over', () => {
  it("begins at this window's own midnight and its Monday, said as moments", () => {
    // A Wednesday evening, local time.
    const evening = new Date(2026, 10, 4, 21, 30)
    const since = spendSince(evening)

    expect(since.dayFrom).toBe(new Date(2026, 10, 4).toISOString())
    expect(since.weekFrom).toBe(new Date(2026, 10, 2).toISOString())
  })

  it('takes a Sunday into the week that began the Monday before', () => {
    const sunday = new Date(2026, 10, 8, 9, 0)
    expect(spendSince(sunday).weekFrom).toBe(new Date(2026, 10, 2).toISOString())
  })

  it('makes the day and the week one moment on a Monday', () => {
    const monday = new Date(2026, 10, 2, 0, 5)
    const since = spendSince(monday)
    expect(since.dayFrom).toBe(since.weekFrom)
  })
})
