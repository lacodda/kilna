import { describe, expect, it } from 'vitest'
import {
  DEFAULT_SORT,
  isNarrowed,
  loadSort,
  narrow,
  saveSort,
  sortRows,
  toggleSort,
  type Sort,
  type SortStore,
} from './catalogue'
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
  released: 0,
  scheduled: 0,
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

describe('gap filters', () => {
  it('finds what nothing has judged', () => {
    const rows = [row({ work_id: 'a', total: null }), row({ work_id: 'b', total: 80 })]
    expect(narrow(rows, { gap: 'unscored' }).map((r) => r.work_id)).toEqual(['a'])
  })

  it('finds judged work that is going nowhere', () => {
    const rows = [
      row({ work_id: 'ready', total: 80 }),
      row({ work_id: 'booked', total: 80, scheduled: 1 }),
      row({ work_id: 'out', total: 80, released: 1 }),
      row({ work_id: 'unjudged', total: null }),
    ]
    // Only the one there is still something to do about: an unjudged work is a
    // different gap, and a released one needs nothing at all.
    expect(narrow(rows, { gap: 'unscheduled' }).map((r) => r.work_id)).toEqual(['ready'])
  })

  it('finds a score that describes an older draft', () => {
    const rows = [row({ work_id: 'a', stale: true }), row({ work_id: 'b' })]
    expect(narrow(rows, { gap: 'stale' }).map((r) => r.work_id)).toEqual(['a'])
  })

  it('counts as narrowing, so an empty result explains itself', () => {
    expect(isNarrowed({ gap: 'stale' })).toBe(true)
    expect(isNarrowed({ tier: 'clip' })).toBe(true)
  })
})

describe('sortRows', () => {
  const scored = (id: string, total: number | null) =>
    row({ work_id: id, title: id, total, tier: total === null ? null : 'clip' })

  it('ranks by the column, both ways', () => {
    const rows = [scored('a', 5), scored('b', 9), scored('c', 7)]
    const desc = sortRows(rows, DEFAULT_SORT).map((r) => r.work_id)
    expect(desc).toEqual(['b', 'c', 'a'])

    const asc: Sort = { column: 'total', direction: 'asc' }
    expect(sortRows(rows, asc).map((r) => r.work_id)).toEqual(['a', 'c', 'b'])
  })

  it('keeps unjudged works last in both directions', () => {
    // The defect this guards: ranking absence with the rest and flipping the
    // sign floated unscored works to the top of a descending sort.
    //
    // The unjudged one is named first in the alphabet on purpose. With a name
    // like "unjudged" it sorted last anyway — comparing against a missing
    // value returns a tie, and the tie-break on title put it there. The test
    // passed for a reason that had nothing to do with the rule.
    const rows = [scored('b', 5), scored('aaa-unjudged', null), scored('c', 9)]
    expect(sortRows(rows, { column: 'total', direction: 'desc' }).at(-1)?.work_id).toBe(
      'aaa-unjudged',
    )
    expect(sortRows(rows, { column: 'total', direction: 'asc' }).at(-1)?.work_id).toBe(
      'aaa-unjudged',
    )
  })

  it('breaks ties by title so the order does not shuffle', () => {
    const rows = [scored('beta', 7), scored('alpha', 7)]
    expect(sortRows(rows, DEFAULT_SORT).map((r) => r.work_id)).toEqual(['alpha', 'beta'])
  })

  it('does not mutate what it was given', () => {
    const rows = [scored('a', 5), scored('b', 9)]
    sortRows(rows, DEFAULT_SORT)
    expect(rows.map((r) => r.work_id)).toEqual(['a', 'b'])
  })

  it('sorts by date, oldest or newest first', () => {
    const rows = [
      row({ work_id: 'old', title: 'old', scored_at: '2026-01-01' }),
      row({ work_id: 'new', title: 'new', scored_at: '2026-08-01' }),
      row({ work_id: 'never', title: 'never', scored_at: null }),
    ]
    expect(sortRows(rows, { column: 'scored', direction: 'desc' }).map((r) => r.work_id)).toEqual([
      'new',
      'old',
      'never',
    ])
  })
})

describe('the remembered sort', () => {
  const store = (initial?: string): SortStore & { value: string | null } => ({
    value: initial ?? null,
    getItem() {
      return this.value
    },
    setItem(_key, value) {
      this.value = value
    },
  })

  it('starts on the strongest work first', () => {
    expect(loadSort(store())).toEqual(DEFAULT_SORT)
  })

  it('comes back as it was left', () => {
    const chosen: Sort = { column: 'title', direction: 'asc' }
    const kept = store()
    saveSort(chosen, kept)
    expect(loadSort(kept)).toEqual(chosen)
  })

  it('falls back rather than trusting whatever is in storage', () => {
    // A column this build does not know would reach code with no case for it.
    expect(loadSort(store(JSON.stringify({ column: 'bpm', direction: 'asc' })))).toEqual(
      DEFAULT_SORT,
    )
    expect(loadSort(store('not json at all'))).toEqual(DEFAULT_SORT)
    expect(loadSort(store(JSON.stringify({ column: 'total' })))).toEqual(DEFAULT_SORT)
  })

  it('does not throw when storage refuses to write', () => {
    // Private windows and full disks both do this; forgetting the sort is the
    // right outcome, a broken screen is not.
    const refusing: SortStore = {
      getItem: () => null,
      setItem: () => {
        throw new Error('quota exceeded')
      },
    }
    expect(() => saveSort(DEFAULT_SORT, refusing)).not.toThrow()
  })
})

describe('toggleSort', () => {
  it('flips the direction of the column already sorted', () => {
    expect(toggleSort({ column: 'total', direction: 'desc' }, 'total')).toEqual({
      column: 'total',
      direction: 'asc',
    })
  })

  it('opens a score column at its highest and a title at its first letter', () => {
    expect(toggleSort(DEFAULT_SORT, 'tier').direction).toBe('desc')
    expect(toggleSort(DEFAULT_SORT, 'title').direction).toBe('asc')
  })
})
