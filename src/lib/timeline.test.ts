import { describe, expect, it } from 'vitest'
import type { Moment } from '@/lib/api/types'
import { axisOf, hasTime, kindOf, localDay, MOMENT_KINDS, type MomentKind } from '@/lib/timeline'

const ALL = new Set<MomentKind>(MOMENT_KINDS)

const version = (at: string, revision = 1): Moment => ({
  type: 'version',
  at,
  id: `v${revision}`,
  role: 'lyrics',
  revision,
  label: null,
  parent: null,
})

const release = (at: string): Moment => ({
  type: 'release',
  at,
  id: 'r1',
  work_id: 'w2',
  title: 'Tide (video)',
  kind: 'youtube',
  released: false,
  time: null,
  url: null,
})

// Noon on the 7th where the window is: the zone the test runs in decides
// the day of a timestamp, so the moments are written in local time too.
const now = new Date(2026, 9, 7, 12, 0, 0)
const local = (day: number, hour: number) => new Date(2026, 9, day, hour, 0, 0).toISOString()

describe('axisOf', () => {
  it('lays moments on days where the window is, latest first', () => {
    const axis = axisOf(
      [version(local(5, 9), 1), version(local(7, 10), 3), version(local(7, 8), 2)],
      ALL,
      now,
    )
    expect(axis.ahead).toEqual([])
    expect(axis.days.map((day) => day.day)).toEqual(['2026-10-07', '2026-10-05'])
    expect(axis.days[0]!.moments.map((m) => (m.type === 'version' ? m.revision : 0))).toEqual([
      3, 2,
    ])
  })

  it('puts what is booked after today ahead, and a release booked today on today', () => {
    const axis = axisOf(
      [release('2026-10-20'), release('2026-10-07'), version(local(7, 9))],
      ALL,
      now,
    )
    expect(axis.ahead).toHaveLength(1)
    expect(axis.days[0]!.day).toBe('2026-10-07')
    expect(axis.days[0]!.moments.map((m) => m.type)).toEqual(['release', 'version'])
  })

  it('lets through only what the filters keep, and always the beginning', () => {
    const axis = axisOf(
      [version(local(6, 9)), release('2026-10-06'), { type: 'begun', at: local(1, 9) }],
      new Set<MomentKind>(['releases']),
      now,
    )
    const types = axis.days.flatMap((day) => day.moments.map((m) => m.type))
    expect(types).toEqual(['release', 'begun'])
  })
})

describe('the pieces of a moment', () => {
  it('reads a bare day as that day, and a timestamp as the local day', () => {
    expect(localDay('2026-10-07')).toBe('2026-10-07')
    expect(localDay(local(3, 23))).toBe('2026-10-03')
    expect(hasTime('2026-10-07')).toBe(false)
    expect(hasTime(local(3, 23))).toBe(true)
  })

  it('files each kind under its filter, the beginning under none', () => {
    expect(kindOf(version(local(1, 1)))).toBe('versions')
    expect(kindOf(release('2026-10-01'))).toBe('releases')
    expect(kindOf({ type: 'begun', at: local(1, 1) })).toBeNull()
  })
})
