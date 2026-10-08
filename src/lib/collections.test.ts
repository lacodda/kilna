import { describe, expect, it } from 'vitest'
import type { Collection, ScoredWork } from '@/lib/api/types'
import { moveTo, progressOf, standingOf } from './collections'

const collection = (over: Partial<Collection> = {}): Collection => ({
  id: 'col',
  profile_id: 'p',
  kind: 'album',
  title: 'Deep time',
  description: null,
  position: 0,
  meta: {},
  target_size: null,
  due_on: null,
  created_at: '2026-09-01T00:00:00Z',
  updated_at: '2026-09-01T00:00:00Z',
  work_ids: [],
  ...over,
})

const row = (work_id: string, total: number | null): ScoredWork => ({
  work_id,
  title: work_id.toUpperCase(),
  kind: 'song',
  status: 'draft',
  total,
  tier: null,
  tier_pinned: false,
  scored_at: total === null ? null : '2026-09-02T00:00:00Z',
  stale: false,
  released: 0,
  scheduled: 0,
  updated_at: '2026-09-02T00:00:00Z',
  created_at: '2026-09-01T00:00:00Z',
  collection_id: 'col',
  tags: [],
  marks: [],
  version_count: 1,
  bookmarked_at: null,
  stage: null,
  publications: [],
})

describe('progressOf', () => {
  it('reads how full a collection is against its target and its day', () => {
    const half = progressOf(
      collection({ work_ids: ['a', 'b', 'c'], target_size: 12, due_on: '2026-12-01' }),
      '2026-11-21',
    )
    expect(half).toMatchObject({ held: 3, target: 12, fraction: 0.25, left: 9, met: false })
    expect(half.daysLeft).toBe(10)
  })

  it('holds a collection fuller than its target at full, with nothing to go', () => {
    const over = progressOf(collection({ work_ids: ['a', 'b', 'c'], target_size: 2 }), '2026-10-01')
    expect(over).toMatchObject({ fraction: 1, left: 0, met: true })
  })

  // A bar with no target to fill would be a fraction made up for it.
  it('has no fraction and nothing to go without a target', () => {
    const open = progressOf(collection({ work_ids: ['a'] }), '2026-10-01')
    expect(open).toMatchObject({ held: 1, target: null, fraction: null, left: null, met: false })
    expect(open.daysLeft).toBeNull()
  })

  it('counts the days past the day it was due as negative', () => {
    expect(progressOf(collection({ due_on: '2026-09-01' }), '2026-09-04').daysLeft).toBe(-3)
  })
})

describe('standingOf', () => {
  const rows = [row('a', 80), row('b', 48), row('c', null), row('d', 91), row('elsewhere', 10)]

  it('averages the judged works and names the weakest and the strongest', () => {
    const standing = standingOf(collection({ work_ids: ['a', 'b', 'c', 'd'] }), rows)

    expect(standing.works.map((work) => work.work_id)).toEqual(['a', 'b', 'c', 'd'])
    expect(standing.scored).toBe(3)
    expect(standing.average).toBeCloseTo((80 + 48 + 91) / 3)
    expect(standing.weakest?.work_id).toBe('b')
    expect(standing.strongest?.work_id).toBe('d')
  })

  // An unjudged work is a question nobody answered, not a zero.
  it('counts an unjudged work towards nothing', () => {
    const standing = standingOf(collection({ work_ids: ['c'] }), rows)
    expect(standing.average).toBeNull()
    expect(standing.scored).toBe(0)
  })

  it('names no weakest link among fewer than two judged works, or among equals', () => {
    expect(standingOf(collection({ work_ids: ['a', 'c'] }), rows).weakest).toBeNull()
    const equals = [row('x', 70), row('y', 70)]
    const tied = standingOf(collection({ work_ids: ['x', 'y'] }), equals)
    expect(tied.weakest).toBeNull()
    expect(tied.strongest).toBeNull()
    expect(tied.average).toBe(70)
  })

  it('gives a tie for weakest to the earlier work, in the collection order', () => {
    const tied = [row('x', 40), row('y', 40), row('z', 90)]
    expect(standingOf(collection({ work_ids: ['y', 'x', 'z'] }), tied).weakest?.work_id).toBe('y')
  })

  it('leaves out a work the catalogue does not hold', () => {
    const standing = standingOf(collection({ work_ids: ['a', 'gone'] }), rows)
    expect(standing.works.map((work) => work.work_id)).toEqual(['a'])
  })
})

describe('moveTo', () => {
  it('moves a work to a place of the resulting list', () => {
    expect(moveTo(['a', 'b', 'c', 'd'], 'a', 2)).toEqual(['b', 'c', 'a', 'd'])
    expect(moveTo(['a', 'b', 'c', 'd'], 'd', 0)).toEqual(['d', 'a', 'b', 'c'])
  })

  it('clamps a place past either end, and leaves a list without the work alone', () => {
    expect(moveTo(['a', 'b'], 'a', 9)).toEqual(['b', 'a'])
    expect(moveTo(['a', 'b'], 'b', -3)).toEqual(['b', 'a'])
    expect(moveTo(['a', 'b'], 'z', 0)).toEqual(['a', 'b'])
  })
})
