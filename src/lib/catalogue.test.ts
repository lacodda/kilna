import { describe, expect, it } from 'vitest'
import {
  ALL_COLUMNS,
  DEFAULT_COLUMNS,
  DEFAULT_SORT,
  groupRows,
  isNarrowed,
  columnsFor,
  columnsForKind,
  withColumns,
  loadColumns,
  sanitizeColumns,
  loadFilter,
  loadSort,
  narrow,
  moveColumn,
  loadWidths,
  saveWidths,
  MIN_COLUMN_WIDTH,
  narrowByColumns,
  isNarrowedByColumns,
  loadColumnFilters,
  saveColumnFilters,
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
  stage: null,
  bookmarked_at: null,
  ...over,
})

describe('narrow', () => {
  it('keeps everything when nothing is set', () => {
    const rows = [row({ work_id: 'a' }), row({ work_id: 'b' })]
    expect(narrow(rows, {})).toHaveLength(2)
  })

  // The release's own question. A row carries a title, not the lyric under it,
  // so a work found by a word in its body can only arrive as an id from the
  // index — and it has to be kept even though the title says nothing.
  it('keeps a work the index matched on text the row does not carry', () => {
    const rows = [row({ work_id: 'kitchen', title: 'Кухня' }), row({ work_id: 'bay', title: 'Гавань' })]

    const kept = narrow(rows, { search: 'холодильник' }, ['kitchen'])

    expect(kept.map((r) => r.work_id)).toEqual(['kitchen'])
  })

  // While the answer is on its way there is no list at all, and the title
  // still has to narrow — otherwise every keystroke would flash the whole
  // catalogue before settling.
  it('narrows on the title alone while the index has not answered', () => {
    const rows = [row({ title: 'Гавань огней' }), row({ title: 'Paper boats' })]

    expect(narrow(rows, { search: 'гавань' }, undefined)).toHaveLength(1)
  })

  // An empty list is an answer — "nothing matched" — and must not read as
  // "no answer yet", or a search for a word nobody wrote would show everything.
  it('narrows to nothing when the index matched nothing', () => {
    // The titles deliberately do NOT contain the word, so that treating the
    // empty answer as "no answer yet" cannot pass by falling back to the title
    // check — the fallback would find nothing here either. The second case is
    // the one that bites: a row whose title matches must still be kept, since
    // either way of matching is enough.
    const rows = [row({ work_id: 'a', title: 'Гавань' }), row({ work_id: 'b', title: 'Paper' })]

    expect(narrow(rows, { search: 'холодильник' }, [])).toHaveLength(0)
    // Nothing in the bodies, but the title says it: kept, and kept because the
    // title said so rather than because the empty list was ignored.
    expect(narrow(rows, { search: 'гавань' }, []).map((r) => r.work_id)).toEqual(['a'])
  })

  // Whether a row survives must not depend on how many others did. With the
  // empty answer misread as "still loading", a search whose only hits are in
  // bodies shows the whole catalogue instead of nothing — the failure is
  // invisible unless a row that should be dropped is put in front of it.
  it('drops a row the index did not match even when it matched others', () => {
    const rows = [
      row({ work_id: 'kitchen', title: 'Кухня' }),
      row({ work_id: 'bay', title: 'Гавань' }),
    ]

    expect(narrow(rows, { search: 'холодильник' }, ['kitchen']).map((r) => r.work_id)).toEqual([
      'kitchen',
    ])
    expect(narrow(rows, { search: 'холодильник' }, []).map((r) => r.work_id)).toEqual([])
  })

  it('narrows to one stage stop', () => {
    const rows = [
      row({ work_id: 'a', stage: 80 }),
      row({ work_id: 'b', stage: 20 }),
      row({ work_id: 'c', stage: null }),
    ]

    expect(narrow(rows, { stage: 80 }).map((r) => r.work_id)).toEqual(['a'])
  })

  // Zero is a judgement, not an absence: "this is a bare idea" is a different
  // answer from "nobody has said", and the unjudged row must not come along.
  it('tells a stage of zero from no stage at all', () => {
    const rows = [row({ work_id: 'idea', stage: 0 }), row({ work_id: 'unsaid', stage: null })]

    expect(narrow(rows, { stage: 0 }).map((r) => r.work_id)).toEqual(['idea'])
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

  // The stage filter was added to `narrow` without being added here, so a
  // catalogue narrowed to one stop showed neither the count nor "clear", and
  // an empty result blamed nothing. Zero is a stop too.
  it('counts a stage filter as narrowing, even the zero stop', () => {
    expect(isNarrowed({ stage: 80 })).toBe(true)
    expect(isNarrowed({ stage: 0 })).toBe(true)
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

  // The stage column sorts since the dial arrived but was left out of the
  // list the reader checks against, so a stage sort was kept and then refused
  // on the next open — a silent reset to the score, with nothing to say why.
  it('remembers a sort by stage', () => {
    const chosen: Sort = { column: 'stage', direction: 'desc' }
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

  it('reads a kind down its own columns and every other kind down the shared list', () => {
    const home = {
      catalogue_columns: ['title', 'total'],
      catalogue_columns_by_kind: { video: ['title', 'versions', 'bpm'] },
    }
    expect(columnsForKind(home, 'video', store())).toEqual({
      columns: ['title', 'versions'],
      fromMachine: false,
    })
    expect(columnsForKind(home, 'song', store())).toEqual({
      columns: ['title', 'total'],
      fromMachine: false,
    })
    expect(columnsForKind(home, undefined, store())).toEqual({
      columns: ['title', 'total'],
      fromMachine: false,
    })
  })

  it('writes a kind its own columns without touching the others', () => {
    const home = {
      catalogue_columns: ['title', 'total'],
      catalogue_columns_by_kind: { song: ['title', 'marks'] },
    }
    expect(withColumns(home, 'video', ['title', 'versions'])).toEqual({
      catalogue_columns_by_kind: { song: ['title', 'marks'], video: ['title', 'versions'] },
    })
    expect(withColumns(home, undefined, ['title', 'id'])).toEqual({
      catalogue_columns: ['title', 'id'],
    })
    expect(withColumns({}, 'video', ['title'])).toEqual({
      catalogue_columns_by_kind: { video: ['title'] },
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

  // The stored order is the drawn order since v0.74, so a toggle must not
  // re-sort the list into the table's own order — that would undo a person's
  // arrangement every time a column came or went.
  it('puts a column turned back on at the end and keeps the rest in place', () => {
    const arranged: ColumnId[] = ['title', 'total', 'stage', 'tier']
    const without = toggleColumn(arranged, 'stage')
    expect(without).toEqual(['title', 'total', 'tier'])
    expect(toggleColumn(without, 'stage')).toEqual(['title', 'total', 'tier', 'stage'])
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

describe('moving a column', () => {
  const arranged: ColumnId[] = ['title', 'stage', 'tier', 'total']

  it('puts the column where it was asked to go, either way', () => {
    expect(moveColumn(arranged, 'total', 1)).toEqual(['title', 'total', 'stage', 'tier'])
    expect(moveColumn(arranged, 'stage', 3)).toEqual(['title', 'tier', 'total', 'stage'])
  })

  it('clamps a move past either end rather than refusing it', () => {
    expect(moveColumn(arranged, 'stage', -1)).toEqual(['stage', 'title', 'tier', 'total'])
    expect(moveColumn(arranged, 'stage', 99)).toEqual(['title', 'tier', 'total', 'stage'])
  })

  it('leaves the list alone for a column it does not hold, or a move to itself', () => {
    expect(moveColumn(arranged, 'id', 0)).toBe(arranged)
    expect(moveColumn(arranged, 'tier', 2)).toBe(arranged)
  })

  it('keeps a stored order as stored, dropping only what it cannot draw', () => {
    expect(sanitizeColumns(['total', 'bpm', 'title', 'stage'])).toEqual(['total', 'title', 'stage'])
  })
})

describe('the remembered widths', () => {
  const store = (initial?: string): SortStore & { value: string | null } => ({
    value: initial ?? null,
    getItem() {
      return this.value
    },
    setItem(_key, value) {
      this.value = value
    },
  })

  it('starts with nothing sized by hand', () => {
    expect(loadWidths(store())).toEqual({})
  })

  it('comes back as it was left', () => {
    const held = store()
    saveWidths({ title: 320, tier: 96 }, held)
    expect(loadWidths(held)).toEqual({ title: 320, tier: 96 })
  })

  it('drops a column this build no longer knows and a width that is not a number', () => {
    const held = store(JSON.stringify({ title: 320, bpm: 80, tier: 'wide', total: null }))
    expect(loadWidths(held)).toEqual({ title: 320 })
  })

  it('never brings back a width narrower than a column can be', () => {
    const held = store(JSON.stringify({ tier: 4 }))
    expect(loadWidths(held)).toEqual({ tier: MIN_COLUMN_WIDTH })
  })

  it('falls back rather than trusting whatever is in storage', () => {
    expect(loadWidths(store('[1, 2]'))).toEqual({})
    expect(loadWidths(store('not json'))).toEqual({})
  })

  it('does not throw when storage refuses to write', () => {
    const refusing: SortStore = {
      getItem: () => null,
      setItem: () => {
        throw new Error('quota exceeded')
      },
    }
    expect(() => saveWidths({ title: 300 }, refusing)).not.toThrow()
  })
})

describe('the header funnels', () => {
  const rows = [
    row({ work_id: 'a', title: 'Гавань огней', stage: 80, tier: 'clip', marks: ['hook'] }),
    row({ work_id: 'b', title: 'Paper boats', stage: 20, tier: 'pic', marks: ['hook', 'bridge'] }),
    row({ work_id: 'c', title: 'Winter shift', stage: null, tier: null, marks: [] }),
  ]
  const ids = (kept: ScoredWork[]) => kept.map((r) => r.work_id)

  it('keeps everything when nothing is ticked or typed', () => {
    expect(narrowByColumns(rows, {})).toHaveLength(3)
    // Empty means "no filter", not "match nothing": unticking the last box
    // must bring the table back.
    expect(narrowByColumns(rows, { title: '  ', stages: [], tiers: [], marks: [] })).toHaveLength(3)
  })

  it('matches part of a title, regardless of case, in Russian too', () => {
    expect(ids(narrowByColumns(rows, { title: 'ГАВАНЬ' }))).toEqual(['a'])
    expect(ids(narrowByColumns(rows, { title: 'boat' }))).toEqual(['b'])
  })

  it('keeps the works standing at any of the ticked stops', () => {
    expect(ids(narrowByColumns(rows, { stages: [20, 80] }))).toEqual(['a', 'b'])
    expect(ids(narrowByColumns(rows, { stages: [40] }))).toEqual([])
  })

  // A work nobody has judged stands at no stop, as `narrow` reads it: it must
  // not come along with whichever stops are ticked.
  it('drops a work with no stage or no tier when those are filtered', () => {
    expect(ids(narrowByColumns(rows, { stages: [20, 80] }))).not.toContain('c')
    expect(ids(narrowByColumns(rows, { tiers: ['clip', 'pic'] }))).not.toContain('c')
  })

  it('keeps the works in any of the ticked tiers', () => {
    expect(ids(narrowByColumns(rows, { tiers: ['pic'] }))).toEqual(['b'])
  })

  it('keeps the works carrying any of the ticked marks', () => {
    expect(ids(narrowByColumns(rows, { marks: ['bridge'] }))).toEqual(['b'])
    expect(ids(narrowByColumns(rows, { marks: ['hook'] }))).toEqual(['a', 'b'])
    expect(ids(narrowByColumns(rows, { marks: ['chorus'] }))).toEqual([])
  })

  it('combines the funnels as AND', () => {
    expect(ids(narrowByColumns(rows, { marks: ['hook'], stages: [20] }))).toEqual(['b'])
  })

  it('counts as narrowing only while a funnel holds something', () => {
    expect(isNarrowedByColumns({})).toBe(false)
    expect(isNarrowedByColumns({ title: ' ', stages: [], tiers: [], marks: [] })).toBe(false)
    expect(isNarrowedByColumns({ marks: ['hook'] })).toBe(true)
    expect(isNarrowedByColumns({ title: 'a' })).toBe(true)
    // The count above the table reads both filters through one question.
    expect(isNarrowed({}, { stages: [20] })).toBe(true)
    expect(isNarrowed({}, {})).toBe(false)
  })

  describe('remembered for the session', () => {
    const store = (initial?: string): SortStore & { value: string | null } => ({
      value: initial ?? null,
      getItem() {
        return this.value
      },
      setItem(_key, value) {
        this.value = value
      },
    })

    it('starts holding nothing', () => {
      expect(loadColumnFilters(store())).toEqual({})
    })

    it('comes back as it was left', () => {
      const held = store()
      saveColumnFilters({ title: 'boat', stages: [20], marks: ['hook'] }, held)
      expect(loadColumnFilters(held)).toEqual({ title: 'boat', stages: [20], marks: ['hook'] })
    })

    it('refuses a value of the wrong shape', () => {
      expect(loadColumnFilters(store(JSON.stringify({ stages: ['polish'] })))).toEqual({})
      expect(loadColumnFilters(store(JSON.stringify({ title: 3 })))).toEqual({})
      expect(loadColumnFilters(store(JSON.stringify({ marks: 'hook' })))).toEqual({})
      expect(loadColumnFilters(store('[1]'))).toEqual({})
      expect(loadColumnFilters(store('not json'))).toEqual({})
    })

    it('does not throw when storage refuses to write', () => {
      const refusing: SortStore = {
        getItem: () => null,
        setItem: () => {
          throw new Error('quota exceeded')
        },
      }
      expect(() => saveColumnFilters({ title: 'a' }, refusing)).not.toThrow()
    })
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

describe('the star', () => {
  it('narrows to the starred, and to nothing else', () => {
    const rows = [
      row({ work_id: 'a', bookmarked_at: '2026-09-12T10:00:00Z' }),
      row({ work_id: 'b' }),
    ]
    expect(narrow(rows, { bookmarked: true }).map((r) => r.work_id)).toEqual(['a'])
    expect(narrow(rows, {})).toHaveLength(2)
  })

  it('counts as narrowing, so an empty result explains itself', () => {
    expect(isNarrowed({ bookmarked: true })).toBe(true)
  })

  it('survives the session, but only as "on"', () => {
    const held = new Map<string, string>()
    const store: SortStore = {
      getItem: (key) => held.get(key) ?? null,
      setItem: (key, value) => void held.set(key, value),
    }
    saveFilter({ bookmarked: true }, store)
    expect(loadFilter(store)).toEqual({ bookmarked: true })
    // A stored `false` is not a shape this build writes; it is read as no
    // filter rather than as a chip that is off.
    store.setItem('kilna.catalogue.filter', JSON.stringify({ bookmarked: false }))
    expect(loadFilter(store)).toEqual({})
  })
})
