import { describe, expect, it } from 'vitest'
import { isNarrowed, narrow } from './catalogue'
import type { ScoredWork } from '@/lib/api'

const row = (over: Partial<ScoredWork>): ScoredWork => ({
  work_id: 'id',
  title: 'Subject',
  kind: 'song',
  status: 'draft',
  total: null,
  tier: null,
  scored_at: null,
  stale: false,
  ...over,
})

describe('narrow', () => {
  it('keeps everything when nothing is set', () => {
    const rows = [row({ work_id: 'a' }), row({ work_id: 'b' })]
    expect(narrow(rows, {})).toHaveLength(2)
  })

  it('matches a title regardless of case, in Russian too', () => {
    const rows = [row({ title: 'Гавань огней' }), row({ title: 'Paper boats' })]
    // The defect this guards: SQLite's LIKE folds ASCII only, so `гавань` found
    // nothing until v0.19 moved the matching out of SQL.
    expect(narrow(rows, { search: 'гавань' })).toHaveLength(1)
    expect(narrow(rows, { search: 'ГАВАНЬ' })).toHaveLength(1)
    expect(narrow(rows, { search: 'PAPER' })).toHaveLength(1)
  })

  it('ignores surrounding whitespace in the query', () => {
    const rows = [row({ title: 'Winter shift' })]
    expect(narrow(rows, { search: '  winter  ' })).toHaveLength(1)
  })

  it('combines filters as AND', () => {
    const rows = [
      row({ work_id: 'a', status: 'draft', kind: 'song' }),
      row({ work_id: 'b', status: 'draft', kind: 'instrumental' }),
      row({ work_id: 'c', status: 'released', kind: 'song' }),
    ]
    const kept = narrow(rows, { status: 'draft', kind: 'song' })
    expect(kept.map((r) => r.work_id)).toEqual(['a'])
  })
})

describe('isNarrowed', () => {
  it('is false for an empty filter', () => {
    expect(isNarrowed({})).toBe(false)
  })

  it('is false for a search box holding only spaces', () => {
    // Otherwise an empty result would blame filters the person did not set.
    expect(isNarrowed({ search: '   ' })).toBe(false)
  })

  it('is true once anything narrows the list', () => {
    expect(isNarrowed({ search: 'a' })).toBe(true)
    expect(isNarrowed({ status: 'draft' })).toBe(true)
    expect(isNarrowed({ kind: 'song' })).toBe(true)
  })
})
