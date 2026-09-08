import { describe, expect, it } from 'vitest'
import {
  ALL_COLUMNS,
  DEFAULT_COLUMNS,
  DEFAULT_SORT,
  groupRows,
  isNarrowed,
  columnsFor,
  loadColumns,
  sanitizeColumns,
  loadFilter,
  loadSort,
  narrow,
  saveColumns,
  saveSort,
  sortRows,
  toggleColumn,
  toggleSort,
  type ColumnId,
  type Sort,
  saveFilter,
  type CatalogueFilter,
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
  updated_at: '2026-08-26T10:00:00Z',
  created_at: '2026-08-20T10:00:00Z',
  collection_id: null,
  tags: [],
  marks: [],
  tier_pinned: false,
  version_count: 0,
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

  it('keeps only what carries the tag', () => {
    const rows = [
      row({ work_id: 'a', tags: ['winter', 'quiet'] }),
      row({ work_id: 'b', tags: ['summer'] }),
      row({ work_id: 'c', tags: [] }),
    ]
    expect(narrow(rows, { tag: 'winter' }).map((r) => r.work_id)).toEqual(['a'])
  })

  it('matches a tag regardless of case, in Russian too', () => {
    const rows = [row({ tags: ['Зима'] })]
    expect(narrow(rows, { tag: 'зима' })).toHaveLength(1)
  })

  // A tag is matched whole. Tags come from what the workspace already holds,
  // so the exact word is the one meant; substring matching would make the
  // narrowest tool in the box the vaguest.
  it('does not take a tag as a prefix of another', () => {
    const rows = [row({ tags: ['winter'] })]
    expect(narrow(rows, { tag: 'win' })).toHaveLength(0)
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
    expect(isNarrowed({ tag: 'winter' })).toBe(true)
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

describe('the remembered filter', () => {
  const store = (initial?: string): SortStore & { value: string | null } => ({
    value: initial ?? null,
    getItem() {
      return this.value
    },
    setItem(_key, value) {
      this.value = value
    },
  })

  it('starts showing everything', () => {
    expect(loadFilter(store())).toEqual({})
  })

  it('comes back as it was left', () => {
    const chosen: CatalogueFilter = { status: 'draft', gap: 'unscored', search: 'winter' }
    const kept = store()
    saveFilter(chosen, kept)
    expect(loadFilter(kept)).toEqual(chosen)
  })

  it('refuses a gap this build cannot switch on', () => {
    // `hasGap` is exhaustive over the three; a fourth would fall through it.
    expect(loadFilter(store(JSON.stringify({ gap: 'unmastered' })))).toEqual({})
  })

  it('keeps a tag it was left holding', () => {
    expect(loadFilter(store(JSON.stringify({ tag: 'winter' })))).toEqual({ tag: 'winter' })
  })

  it('refuses a value of the wrong shape', () => {
    expect(loadFilter(store(JSON.stringify({ status: 7 })))).toEqual({})
    expect(loadFilter(store(JSON.stringify({ tag: 7 })))).toEqual({})
    expect(loadFilter(store(JSON.stringify({ search: ['a'] })))).toEqual({})
    expect(loadFilter(store('not json at all'))).toEqual({})
    expect(loadFilter(store(JSON.stringify(null)))).toEqual({})
  })

  it('refuses a value that is not an object at all', () => {
    // An array is the one that gets through a plain `typeof` check: every
    // field of it reads as `undefined`, so it passes every test for a field
    // and comes back as a filter. It was doing exactly that.
    expect(loadFilter(store('[1,2]'))).toEqual({})
    expect(loadFilter(store('42'))).toEqual({})
    expect(loadFilter(store('"hello"'))).toEqual({})
  })

  it('keeps a status the profile no longer has', () => {
    // Unlike a gap, a status is the profile's own word: one that has been
    // renamed narrows to nothing, which the empty state already explains, and
    // dropping it silently would be the more confusing answer.
    expect(loadFilter(store(JSON.stringify({ status: 'retired' })))).toEqual({
      status: 'retired',
    })
  })

  it('does not throw when storage refuses to write', () => {
    const refusing: SortStore = {
      getItem: () => null,
      setItem: () => {
        throw new Error('quota exceeded')
      },
    }
    expect(() => saveFilter({ gap: 'stale' }, refusing)).not.toThrow()
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

describe('the shown columns', () => {
  const store = (initial?: string): SortStore & { value: string | null } => ({
    value: initial ?? null,
    getItem() {
      return this.value
    },
    setItem(_key, value) {
      this.value = value
    },
  })

  it('starts on the default set, without the identifier', () => {
    expect(loadColumns(store())).toEqual(DEFAULT_COLUMNS)
    expect(loadColumns(store())).not.toContain('id')
  })

  it('comes back exactly as it was saved', () => {
    // Reading does not reorder: the table order is imposed by `toggleColumn`,
    // where a person's click needs a predictable place to land.
    const held = store()
    saveColumns(['title', 'id', 'total'], held)
    expect(loadColumns(held)).toEqual(['title', 'id', 'total'])
  })

  it('drops a column this build no longer knows, keeping the rest', () => {
    const held = store(JSON.stringify(['title', 'bpm', 'total']))
    expect(loadColumns(held)).toEqual(['title', 'total'])
  })

  it('puts the title back when a stored set left it out', () => {
    const held = store(JSON.stringify(['total', 'tier']))
    expect(loadColumns(held)).toContain('title')
  })

  it('opens on the profile columns when the profile has any', () => {
    const held = store(JSON.stringify(['title', 'id']))
    const opened = columnsFor(['title', 'total', 'bpm'], held)
    expect(opened).toEqual({ columns: ['title', 'total'], fromMachine: false })
  })

  it('opens on what the machine remembered until the profile has columns', () => {
    const held = store(JSON.stringify(['title', 'id']))
    expect(columnsFor(null, held)).toEqual({ columns: ['title', 'id'], fromMachine: true })
    expect(columnsFor(undefined, store())).toEqual({
      columns: DEFAULT_COLUMNS,
      fromMachine: true,
    })
  })

  it('treats an empty profile list like a corrupt one, not like "hide everything"', () => {
    expect(sanitizeColumns([])).toEqual(DEFAULT_COLUMNS)
    expect(sanitizeColumns('title')).toEqual(DEFAULT_COLUMNS)
  })

  it('falls back to the default when nothing stored is recognisable', () => {
    const held = store(JSON.stringify(['bpm', 'mood']))
    expect(loadColumns(held)).toEqual(DEFAULT_COLUMNS)
  })

  it('falls back when the stored value is not a list at all', () => {
    // The same shape that got past the filter guard in v0.46: an array is
    // `typeof 'object'`, and an object is not an array.
    expect(loadColumns(store('{"title":true}'))).toEqual(DEFAULT_COLUMNS)
    expect(loadColumns(store('not json'))).toEqual(DEFAULT_COLUMNS)
  })

  it('survives storage that refuses to be written', () => {
    const refusing: SortStore = {
      getItem: () => null,
      setItem: () => {
        throw new Error('quota exceeded')
      },
    }
    expect(() => saveColumns(DEFAULT_COLUMNS, refusing)).not.toThrow()
  })
})

describe('toggling a column', () => {
  it('turns one off and back on', () => {
    const without = toggleColumn(DEFAULT_COLUMNS, 'total')
    expect(without).not.toContain('total')
    expect(toggleColumn(without, 'total')).toContain('total')
  })

  it('restores it to the table order, not to the end', () => {
    const without = toggleColumn(DEFAULT_COLUMNS, 'tier')
    const back = toggleColumn(without, 'tier')
    expect(back).toEqual(DEFAULT_COLUMNS)
  })

  it('refuses to hide the title', () => {
    expect(toggleColumn(DEFAULT_COLUMNS, 'title')).toEqual(DEFAULT_COLUMNS)
  })

  it('never invents a column the table cannot draw', () => {
    const shown = ALL_COLUMNS.reduce<ColumnId[]>((held, id) => toggleColumn(held, id), [
      ...DEFAULT_COLUMNS,
    ])
    expect(shown.every((id) => ALL_COLUMNS.includes(id))).toBe(true)
  })
})

describe('grouping', () => {
  it('leaves one block when it is off', () => {
    const rows = [row({ work_id: 'a' }), row({ work_id: 'b' })]
    expect(groupRows(rows, 'none')).toEqual([{ key: null, rows }])
  })

  it('gathers rows sharing a status', () => {
    const rows = [
      row({ work_id: 'a', status: 'draft' }),
      row({ work_id: 'b', status: 'ready' }),
      row({ work_id: 'c', status: 'draft' }),
    ]
    const blocks = groupRows(rows, 'status')
    expect(blocks).toHaveLength(2)
    expect(blocks[0]).toEqual({
      key: 'draft',
      rows: [rows[0], rows[2]],
    })
  })

  it('keeps the order the sort gave, so the leading block holds the best work', () => {
    const rows = [
      row({ work_id: 'best', tier: 'clip' }),
      row({ work_id: 'rest', tier: 'hold' }),
      row({ work_id: 'also', tier: 'clip' }),
    ]
    expect(groupRows(rows, 'tier').map((block) => block.key)).toEqual(['clip', 'hold'])
  })

  it('puts the works without a value in a block of their own, last', () => {
    const rows = [
      row({ work_id: 'unjudged', tier: null }),
      row({ work_id: 'judged', tier: 'clip' }),
    ]
    const blocks = groupRows(rows, 'tier')
    expect(blocks.at(-1)).toEqual({ key: null, rows: [rows[0]] })
  })

  it('treats an empty string as no value rather than as a block named ""', () => {
    const rows = [row({ work_id: 'blank', tier: '' })]
    expect(groupRows(rows, 'tier')).toEqual([{ key: null, rows }])
  })

  it('loses no row, whatever the grouping', () => {
    const rows = [
      row({ work_id: 'a', status: 'draft', tier: 'clip' }),
      row({ work_id: 'b', status: 'ready', tier: null }),
      row({ work_id: 'c', status: 'draft', tier: '' }),
    ]
    for (const by of ['none', 'status', 'tier'] as const) {
      const held = groupRows(rows, by).flatMap((block) => block.rows)
      expect(held).toHaveLength(rows.length)
    }
  })
})
