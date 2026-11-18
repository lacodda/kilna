import { describe, expect, it } from 'vitest'
import {
  ALL_COLUMNS,
  andBlocked,
  bookingOf,
  clearedColumn,
  columnsFor,
  columnsForKind,
  countGaps,
  countRepeats,
  DEFAULT_COLUMNS,
  DEFAULT_SORT,
  EMPTY_TICK,
  groupRows,
  isNarrowed,
  isNarrowedByColumns,
  loadColumnFilters,
  loadColumnsChosen,
  loadFilter,
  loadSort,
  loadWidths,
  logicOf,
  MIN_COLUMN_WIDTH,
  moveColumn,
  narrow,
  narrowByColumns,
  sanitizeColumns,
  saveColumnFilters,
  saveColumnsChosen,
  saveFilter,
  saveSort,
  saveWidths,
  sortRows,
  tierRanges,
  toggleColumn,
  toggleSort,
  withColumns,
  withIntroduced,
  withLogic,
  withNextReleases,
  withTicks,
  type CatalogueFilter,
  type CatalogueRow,
  type ColumnId,
  type Sort,
  type SortStore,
} from './catalogue'
import type { ScheduledRelease, ScoredWork } from '@/lib/api/types'

const row = (over: Partial<CatalogueRow>): CatalogueRow => ({
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
  stage: 0,
  bookmarked_at: null,
  publications: [],
  next_release: null,
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
    const rows = [
      row({ work_id: 'kitchen', title: 'Кухня' }),
      row({ work_id: 'bay', title: 'Гавань' }),
    ]

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
      row({ work_id: 'a', stage: 75 }),
      row({ work_id: 'b', stage: 25 }),
      row({ work_id: 'c', stage: 0 }),
    ]

    expect(narrow(rows, { stage: 75 }).map((r) => r.work_id)).toEqual(['a'])
    // The first stop is a stop: `0` is falsy, and a filter that dropped it
    // would make the works nobody has touched the only ones not to be found.
    expect(narrow(rows, { stage: 0 }).map((r) => r.work_id)).toEqual(['c'])
    // A line written when the scale had seven stops reads as the stop it
    // lands on (v0.119).
    expect(narrow(rows, { stage: 83 }).map((r) => r.work_id)).toEqual(['a'])
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

  it('counts a collection as narrowing', () => {
    expect(isNarrowed({ collection: 'col-deep' })).toBe(true)
  })
})

describe('a collection in the catalogue', () => {
  const rows = [
    row({ work_id: 'a', collection_id: 'col-deep' }),
    row({ work_id: 'b', collection_id: null }),
    row({ work_id: 'c', collection_id: 'col-winter' }),
    row({ work_id: 'd', collection_id: 'col-deep' }),
  ]

  it('narrows to the works it holds', () => {
    expect(narrow(rows, { collection: 'col-deep' }).map((r) => r.work_id)).toEqual(['a', 'd'])
  })

  it('gathers the rows into a block per collection, the loose ones last', () => {
    const blocks = groupRows(rows, 'collection')
    expect(blocks.map((block) => [block.key, block.rows.map((r) => r.work_id)])).toEqual([
      ['col-deep', ['a', 'd']],
      ['col-winter', ['c']],
      [null, ['b']],
    ])
  })

  // Kept for the session like every other narrowing, and read back as one.
  it('survives a visit to a card and back', () => {
    const held = new Map<string, string>()
    const kept: SortStore = {
      getItem: (key) => held.get(key) ?? null,
      setItem: (key, value) => void held.set(key, value),
    }
    saveFilter({ collection: 'col-deep' }, kept)
    expect(loadFilter(kept)).toEqual({ collection: 'col-deep' })
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

  it('sorts by the next release, with nothing booked last either way', () => {
    const booked = (id: string, day: string | null) =>
      row({
        work_id: id,
        title: id,
        next_release:
          day === null ? null : release({ id: `r-${id}`, work_id: id, scheduled_at: day }),
      })
    // Named so the tie-break on title would put the unbooked one first: the
    // rule has to be what keeps it last.
    const rows = [booked('b', '2026-10-02'), booked('a-unbooked', null), booked('c', '2026-09-20')]
    const asc = sortRows(rows, { column: 'release', direction: 'asc' }).map((r) => r.work_id)
    const desc = sortRows(rows, { column: 'release', direction: 'desc' }).map((r) => r.work_id)
    expect(asc).toEqual(['c', 'b', 'a-unbooked'])
    expect(desc).toEqual(['b', 'c', 'a-unbooked'])
  })

  it('sorts by date, oldest or newest first', () => {
    const rows = [
      row({ work_id: 'old', title: 'old', updated_at: '2026-01-01' }),
      row({ work_id: 'new', title: 'new', updated_at: '2026-08-01' }),
      row({ work_id: 'never', title: 'never', updated_at: '2025-01-01' }),
    ]
    expect(sortRows(rows, { column: 'updated', direction: 'desc' }).map((r) => r.work_id)).toEqual([
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
    expect(toggleSort(DEFAULT_SORT, 'versions').direction).toBe('desc')
    expect(toggleSort(DEFAULT_SORT, 'title').direction).toBe('asc')
  })

  it('opens the next release at the soonest', () => {
    expect(toggleSort(DEFAULT_SORT, 'release').direction).toBe('asc')
  })
})

describe('the shown columns', () => {
  it('opens on the default set, without the identifier, when the profile names none', () => {
    expect(columnsFor(null)).toEqual(DEFAULT_COLUMNS)
    expect(columnsFor(undefined)).toEqual(DEFAULT_COLUMNS)
    expect(columnsFor(undefined)).not.toContain('id')
  })

  it('opens on the profile columns exactly as they are kept', () => {
    // Reading does not reorder: the table order is imposed by `toggleColumn`,
    // where a person's click needs a predictable place to land.
    expect(columnsFor(['title', 'id', 'total'])).toEqual(['title', 'id', 'total'])
  })

  it('drops a column this build no longer knows, keeping the rest', () => {
    expect(columnsFor(['title', 'bpm', 'total'])).toEqual(['title', 'total'])
  })

  it('puts the title back when a kept set left it out', () => {
    expect(columnsFor(['total', 'versions'])).toContain('title')
  })

  it('reads a kind down its own columns and every other kind down the shared list', () => {
    const home = {
      catalogue_columns: ['title', 'total'],
      catalogue_columns_by_kind: { video: ['title', 'versions', 'bpm'] },
    }
    expect(columnsForKind(home, 'video')).toEqual(['title', 'versions'])
    expect(columnsForKind(home, 'song')).toEqual(['title', 'total'])
    expect(columnsForKind(home, undefined)).toEqual(['title', 'total'])
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

  it('falls back to the default when nothing kept is recognisable', () => {
    expect(columnsFor(['bpm', 'mood'])).toEqual(DEFAULT_COLUMNS)
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
    const arranged: ColumnId[] = ['title', 'total', 'stage', 'versions']
    const without = toggleColumn(arranged, 'stage')
    expect(without).toEqual(['title', 'total', 'versions'])
    expect(toggleColumn(without, 'stage')).toEqual(['title', 'total', 'versions', 'stage'])
  })

  it('refuses to hide the title', () => {
    expect(toggleColumn(DEFAULT_COLUMNS, 'title')).toEqual(DEFAULT_COLUMNS)
  })

  it('never invents a column the table cannot draw', () => {
    const shown = ALL_COLUMNS.reduce<ColumnId[]>(
      (held, id) => toggleColumn(held, id),
      [...DEFAULT_COLUMNS],
    )
    expect(shown.every((id) => ALL_COLUMNS.includes(id))).toBe(true)
  })
})

describe('moving a column', () => {
  const arranged: ColumnId[] = ['title', 'stage', 'versions', 'total']

  it('puts the column where it was asked to go, either way', () => {
    expect(moveColumn(arranged, 'total', 1)).toEqual(['title', 'total', 'stage', 'versions'])
    expect(moveColumn(arranged, 'stage', 3)).toEqual(['title', 'versions', 'total', 'stage'])
  })

  it('clamps a move past either end rather than refusing it', () => {
    expect(moveColumn(arranged, 'stage', -1)).toEqual(['stage', 'title', 'versions', 'total'])
    expect(moveColumn(arranged, 'stage', 99)).toEqual(['title', 'versions', 'total', 'stage'])
  })

  it('leaves the list alone for a column it does not hold, or a move to itself', () => {
    expect(moveColumn(arranged, 'id', 0)).toBe(arranged)
    expect(moveColumn(arranged, 'versions', 2)).toBe(arranged)
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
    saveWidths({ title: 320, total: 96 }, held)
    expect(loadWidths(held)).toEqual({ title: 320, total: 96 })
  })

  it('drops a column this build no longer knows and a width that is not a number', () => {
    const held = store(JSON.stringify({ title: 320, bpm: 80, tier: 'wide', total: null }))
    expect(loadWidths(held)).toEqual({ title: 320 })
  })

  it('never brings back a width narrower than a column can be', () => {
    const held = store(JSON.stringify({ total: 4 }))
    expect(loadWidths(held)).toEqual({ total: MIN_COLUMN_WIDTH })
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

describe('what the works feel like (FR-02)', () => {
  const rows = [
    row({ work_id: 'song', vibe: { vibe: { mood: 'dark', character: 'edgy' } } }),
    // A clip shows its song's.
    row({ work_id: 'clip', vibe: { vibe: { mood: 'dark', character: 'edgy' }, from: 'Song' } }),
    row({ work_id: 'bright', vibe: { vibe: { mood: 'warm', complexity: 'simple' } } }),
    row({ work_id: 'unsaid' }),
  ]
  const ids = (kept: ScoredWork[]) => kept.map((r) => r.work_id)

  it('narrows by a word on a line, a taken vibe counting as its own', () => {
    expect(ids(narrow(rows, { mood: 'dark' }))).toEqual(['song', 'clip'])
    expect(ids(narrow(rows, { mood: 'dark', character: 'calm' }))).toEqual([])
    expect(ids(narrow(rows, { complexity: 'simple' }))).toEqual(['bright'])
    expect(isNarrowed({ character: 'edgy' })).toBe(true)
  })

  it('ticks any word of a line, both of two lines, and the unsaid beside them', () => {
    expect(ids(narrowByColumns(rows, { vibe: ['mood:dark', 'mood:warm'] }))).toEqual([
      'song',
      'clip',
      'bright',
    ])
    expect(
      ids(narrowByColumns(rows, { vibe: ['mood:dark', 'mood:warm', 'complexity:simple'] })),
    ).toEqual(['bright'])
    expect(ids(narrowByColumns(rows, { vibe: ['none'] }))).toEqual(['unsaid'])
    expect(ids(narrowByColumns(rows, { vibe: ['none', 'mood:warm'] }))).toEqual([
      'bright',
      'unsaid',
    ])
    expect(narrowByColumns(rows, { vibe: [] })).toHaveLength(4)
    expect(isNarrowedByColumns({ vibe: ['none'] })).toBe(true)
  })
})

describe('the header funnels', () => {
  const rows = [
    row({ work_id: 'a', title: 'Гавань огней', stage: 75, tier: 'clip', marks: ['hook'] }),
    row({
      work_id: 'b',
      title: 'Paper boats',
      status: 'scored',
      stage: 25,
      tier: 'pic',
      marks: ['hook', 'bridge'],
    }),
    row({ work_id: 'c', title: 'Winter shift', stage: 0, tier: null, marks: [] }),
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
    expect(ids(narrowByColumns(rows, { stages: [25, 75] }))).toEqual(['a', 'b'])
    expect(ids(narrowByColumns(rows, { stages: [50] }))).toEqual([])
  })

  it('drops a work with no tier when tiers are filtered', () => {
    expect(ids(narrowByColumns(rows, { tiers: ['clip', 'pic'] }))).not.toContain('c')
  })

  it('finds the works nobody has touched at the first stop', () => {
    // What the owner went looking for: until v0.119 these held no stage at
    // all and had an entry of their own in the funnel; they stand at 0 now.
    expect(ids(narrowByColumns(rows, { stages: [0] }))).toEqual(['c'])
    expect(ids(narrowByColumns(rows, { stages: [0, 75] }))).toEqual(['a', 'c'])
  })

  it('reads what a view saved on the older scale ticked as the stops it lands on', () => {
    // -1 was the "no stage" tick; 20 and 80 were stops of the scale before.
    expect(ids(narrowByColumns(rows, { stages: [-1] }))).toEqual(['c'])
    expect(ids(narrowByColumns(rows, { stages: [20, 80] }))).toEqual(['a', 'b'])
  })

  it('slices by what the guard of repeats marks, and by what it marks red', () => {
    // The marks are the guard's, by work: `a` repeats red, `b` orange, `c`
    // nothing.
    const repeats = new Map([
      ['a', { level: 'red' as const }],
      ['b', { level: 'orange' as const }],
    ])

    expect(ids(narrow(rows, { repeats: 'any' }, undefined, repeats))).toEqual(['a', 'b'])
    expect(ids(narrow(rows, { repeats: 'red' }, undefined, repeats))).toEqual(['a'])
    expect(ids(narrow(rows, {}, undefined, repeats))).toEqual(['a', 'b', 'c'])
    // Before the marks arrive nothing is known to repeat: the slice is empty
    // rather than everything.
    expect(narrow(rows, { repeats: 'any' })).toEqual([])

    expect(countRepeats(rows, repeats)).toEqual({ any: 2, red: 1 })
    expect(isNarrowed({ repeats: 'red' })).toBe(true)
  })

  // v0.139, FR-12 12.2: with or without, any or all, and the empty tick.
  it('reads the ticks of a column of many values all four ways', () => {
    const marks = (combine: 'or' | 'and', polarity: 'include' | 'exclude') =>
      ids(
        narrowByColumns(rows, {
          marks: ['hook', 'bridge'],
          logic: { marks: { combine, polarity } },
        }),
      )
    expect(marks('or', 'include')).toEqual(['a', 'b'])
    expect(marks('and', 'include')).toEqual(['b'])
    expect(marks('or', 'exclude')).toEqual(['c'])
    expect(marks('and', 'exclude')).toEqual(['a', 'c'])
  })

  it('finds the works with nothing in the column, and the rest without', () => {
    expect(ids(narrowByColumns(rows, { marks: [EMPTY_TICK] }))).toEqual(['c'])
    expect(ids(narrowByColumns(rows, { tiers: [EMPTY_TICK, 'pic'] }))).toEqual(['b', 'c'])
    const without = { tiers: [EMPTY_TICK], logic: { tiers: { polarity: 'exclude' as const } } }
    expect(ids(narrowByColumns(rows, without))).toEqual(['a', 'b'])
  })

  it('reads the title by its words, any of them or all', () => {
    expect(ids(narrowByColumns(rows, { title: 'paper winter' }))).toEqual(['b', 'c'])
    const all = { title: 'paper boats', logic: { title: { combine: 'and' as const } } }
    expect(ids(narrowByColumns(rows, all))).toEqual(['b'])
    const none = { title: 'paper winter', logic: { title: { polarity: 'exclude' as const } } }
    expect(ids(narrowByColumns(rows, none))).toEqual(['a'])
  })

  it('reads "and" over two ticks of one value as "or", and drops the ticked without', () => {
    expect(andBlocked('stages', 2)).toBe(true)
    expect(andBlocked('stages', 1)).toBe(false)
    expect(andBlocked('marks', 2)).toBe(false)
    const both = { stages: [25, 75], logic: { stages: { combine: 'and' as const } } }
    expect(ids(narrowByColumns(rows, both))).toEqual(['a', 'b'])
    const not = { stages: [25, 75], logic: { stages: { polarity: 'exclude' as const } } }
    expect(ids(narrowByColumns(rows, not))).toEqual(['c'])
  })

  it('lets go of "and" at a second tick of one value, and clears back to the start', () => {
    const anded = withLogic({ stages: [25] }, 'stages', { combine: 'and' })
    expect(logicOf(anded, 'stages').combine).toBe('and')
    expect(logicOf(withTicks(anded, 'stages', [25, 75]), 'stages').combine).toBe('or')
    const without = withLogic({ marks: ['hook'] }, 'marks', { polarity: 'exclude', combine: 'and' })
    const cleared = clearedColumn(without, 'marks')
    expect(cleared.marks).toBeUndefined()
    expect(cleared.logic).toBeUndefined()
    expect(logicOf(cleared, 'marks')).toEqual({ polarity: 'include', combine: 'or' })
  })

  it('keeps the works in any of the ticked tiers', () => {
    expect(ids(narrowByColumns(rows, { tiers: ['pic'] }))).toEqual(['b'])
  })

  // The toolbar's status dropdown went in v0.79; the funnel on the Status
  // column is where a status is ticked now, and it takes more than one.
  it('keeps the works standing at any of the ticked statuses', () => {
    expect(ids(narrowByColumns(rows, { statuses: ['scored'] }))).toEqual(['b'])
    expect(ids(narrowByColumns(rows, { statuses: ['draft', 'scored'] }))).toEqual(['a', 'b', 'c'])
    expect(ids(narrowByColumns(rows, { statuses: ['released'] }))).toEqual([])
    expect(isNarrowedByColumns({ statuses: ['draft'] })).toBe(true)
    expect(isNarrowedByColumns({ statuses: [] })).toBe(false)
  })

  it('keeps the works carrying any of the ticked marks', () => {
    expect(ids(narrowByColumns(rows, { marks: ['bridge'] }))).toEqual(['b'])
    expect(ids(narrowByColumns(rows, { marks: ['hook'] }))).toEqual(['a', 'b'])
    expect(ids(narrowByColumns(rows, { marks: ['chorus'] }))).toEqual([])
  })

  it('combines the funnels as AND', () => {
    expect(ids(narrowByColumns(rows, { marks: ['hook'], stages: [25] }))).toEqual(['b'])
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
      saveColumnFilters({ title: 'boat', statuses: ['draft'], stages: [20], marks: ['hook'] }, held)
      expect(loadColumnFilters(held)).toEqual({
        title: 'boat',
        statuses: ['draft'],
        stages: [20],
        marks: ['hook'],
      })
    })

    it('refuses a value of the wrong shape', () => {
      expect(loadColumnFilters(store(JSON.stringify({ stages: ['polish'] })))).toEqual({})
      expect(loadColumnFilters(store(JSON.stringify({ title: 3 })))).toEqual({})
      expect(loadColumnFilters(store(JSON.stringify({ marks: 'hook' })))).toEqual({})
      expect(loadColumnFilters(store(JSON.stringify({ statuses: [1] })))).toEqual({})
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

/** A release as the calendar hands it over, booked unless told otherwise. */
function release(
  over: Partial<ScheduledRelease> & Pick<ScheduledRelease, 'id' | 'work_id'>,
): ScheduledRelease {
  return {
    kind: 'audio',
    status: 'planned',
    scheduled_at: '2026-09-22',
    released_at: null,
    url: null,
    slot_pinned_at: null,
    scheduled_time: null,
    time_zone: null,
    meta: {},
    created_at: '2026-09-01T10:00:00Z',
    updated_at: '2026-09-01T10:00:00Z',
    work_title: 'Subject',
    work_kind: 'song',
    total: null,
    tier: null,
    readiness: { checks: [], ready: true },
    work_stage: 0,
    ...over,
  }
}

describe('the next release', () => {
  const next = (rows: ScoredWork[], releases: ScheduledRelease[] | undefined) =>
    withNextReleases(rows, releases).map((r) => [r.work_id, r.next_release?.id ?? null])

  it('takes the earliest booking of each work', () => {
    const releases = [
      release({ id: 'late', work_id: 'a', scheduled_at: '2026-10-01' }),
      release({ id: 'soon', work_id: 'a', scheduled_at: '2026-09-20' }),
      release({ id: 'other', work_id: 'b', scheduled_at: '2026-09-18' }),
    ]
    expect(next([row({ work_id: 'a' }), row({ work_id: 'b' })], releases)).toEqual([
      ['a', 'soon'],
      ['b', 'other'],
    ])
  })

  // What went out is history, not a booking, and a release with no day is in
  // the queue rather than on the calendar: neither says when the work goes
  // out next.
  it('passes over what has gone out and what holds no day', () => {
    const releases = [
      release({ id: 'out', work_id: 'a', status: 'released', scheduled_at: '2026-09-01' }),
      release({ id: 'queued', work_id: 'a', scheduled_at: null }),
      release({ id: 'booked', work_id: 'a', scheduled_at: '2026-11-05' }),
    ]
    expect(next([row({ work_id: 'a' })], releases)).toEqual([['a', 'booked']])
  })

  it('keeps a booking whose day passed without it going out', () => {
    // The slot was missed, not kept: it is still the next thing owed.
    const releases = [release({ id: 'missed', work_id: 'a', scheduled_at: '2026-08-01' })]
    expect(next([row({ work_id: 'a' })], releases)).toEqual([['a', 'missed']])
  })

  it('settles two bookings on one day by the hour, the unset hour first', () => {
    const releases = [
      release({ id: 'evening', work_id: 'a', scheduled_time: '19:00' }),
      release({ id: 'morning', work_id: 'a', scheduled_time: '09:30' }),
    ]
    expect(next([row({ work_id: 'a' })], releases)).toEqual([['a', 'morning']])
    const unset = [...releases, release({ id: 'whenever', work_id: 'a' })]
    expect(next([row({ work_id: 'a' })], unset)).toEqual([['a', 'whenever']])
  })

  it('gives every row none while the calendar is still being read', () => {
    expect(next([row({ work_id: 'a' }), row({ work_id: 'b' })], undefined)).toEqual([
      ['a', null],
      ['b', null],
    ])
  })
})

describe('the gap counts', () => {
  it('counts each gap over every row, a row in more than one gap in each', () => {
    const rows = [
      row({ work_id: 'unjudged', total: null }),
      row({ work_id: 'waiting', total: 7 }),
      row({ work_id: 'stale-waiting', total: 6, stale: true }),
      row({ work_id: 'booked', total: 8, scheduled: 1 }),
      row({ work_id: 'out', total: 9, released: 1 }),
    ]
    expect(countGaps(rows)).toEqual({ unscored: 1, unscheduled: 2, stale: 1 })
  })

  it('counts nothing in an empty catalogue', () => {
    expect(countGaps([])).toEqual({ unscored: 0, unscheduled: 0, stale: 0 })
  })
})

describe('the columns v0.79 introduced', () => {
  it('show in a list stored before them, each after the column it follows', () => {
    const stored: ColumnId[] = ['title', 'stage', 'marks', 'versions', 'total']
    // Status follows marks, as in the default; the release columns follow
    // versions, however the person ordered the rest.
    // The vibe (v0.124) follows the stage, where the default has it.
    expect(withIntroduced(stored, false)).toEqual([
      'title',
      'stage',
      'vibe',
      'marks',
      'status',
      'versions',
      'release',
      'total',
    ])
  })

  it('show a later column in a list that knew the earlier ones, and only that', () => {
    // Written knowing of Status and without the next release: it was hidden
    // on purpose and stays so; the vibe did not exist yet, and shows.
    const stored: ColumnId[] = ['title', 'status', 'stage', 'total']
    expect(withIntroduced(stored, false)).toEqual(['title', 'status', 'stage', 'vibe', 'total'])
    // Named already, it is where the person put it.
    const placed: ColumnId[] = ['vibe', 'title', 'status']
    expect(withIntroduced(placed, false)).toEqual(placed)
  })

  it('stay as the person left them once a choice was made', () => {
    const stored: ColumnId[] = ['title', 'stage', 'total']
    expect(withIntroduced(stored, true)).toEqual(stored)
  })

  it('are not forced into a list that already names one of them', () => {
    // Chosen knowing them: the other two were hidden on purpose. What came
    // after them (the vibe, v0.124) was not, and shows.
    const stored: ColumnId[] = ['title', 'release', 'total']
    expect(withIntroduced(stored, false)).toEqual(['title', 'vibe', 'release', 'total'])
  })

  it('remember the choice on this machine', () => {
    const items = new Map<string, string>()
    const store: SortStore = {
      getItem: (key) => items.get(key) ?? null,
      setItem: (key, value) => void items.set(key, value),
    }
    expect(loadColumnsChosen(store)).toBe(false)
    saveColumnsChosen(store)
    expect(loadColumnsChosen(store)).toBe(true)
  })
})

// v0.140, FR-12 12.3: the merged columns and the new ones.
describe('the columns of v0.140', () => {
  const plan = (work: string, day: string) => ({
    id: `p-${work}`,
    work_id: work,
    scheduled_at: day,
    note: null,
    created_at: '',
    updated_at: '',
  })

  it('says where the next release stands by the day the cell shows', () => {
    const on = (day: string | null, planned?: string) =>
      bookingOf(
        {
          next_release: day === null ? null : release({ id: 'r', work_id: 'w', scheduled_at: day }),
          plan: planned === undefined ? null : plan('w', planned),
        },
        '2026-10-10',
      )
    expect(on('2026-10-12')).toBe('planned')
    expect(on('2026-10-10')).toBe('planned')
    expect(on('2026-10-01')).toBe('overdue')
    expect(on(null)).toBe('undated')
    // Nothing booked: the planned slot is the date.
    expect(on(null, '2026-10-20')).toBe('planned')
    expect(on(null, '2026-10-02')).toBe('overdue')
  })

  it('letters the tiers by rank, each a range of the score', () => {
    const ranges = tierRanges([
      { key: 'hold', min: 0 },
      { key: 'clip', min: 78 },
      { key: 'audio', min: 55 },
    ])
    expect(ranges.map(({ tier, letter, from, below }) => [tier.key, letter, from, below])).toEqual([
      ['clip', 'S', 78, null],
      ['audio', 'A', 55, 78],
      ['hold', 'B', 0, 55],
    ])
  })

  it('narrows by the next release, the collection and the tags', () => {
    const rows = [
      row({ work_id: 'a', collection_id: 'album', tags: ['winter', 'night'] }),
      row({
        work_id: 'b',
        tags: ['winter'],
        next_release: release({ id: 'r-b', work_id: 'b', scheduled_at: '2026-09-01' }),
      }),
      row({ work_id: 'c', tags: [], plan: plan('c', '2026-12-01') }),
    ]
    const ids = (kept: CatalogueRow[]) => kept.map((one) => one.work_id)
    const day = '2026-10-10'
    expect(ids(narrowByColumns(rows, { booking: ['overdue'] }, day))).toEqual(['b'])
    expect(ids(narrowByColumns(rows, { booking: ['planned'] }, day))).toEqual(['c'])
    expect(ids(narrowByColumns(rows, { booking: ['undated'] }, day))).toEqual(['a'])
    expect(ids(narrowByColumns(rows, { collections: ['album'] }, day))).toEqual(['a'])
    expect(ids(narrowByColumns(rows, { collections: [EMPTY_TICK] }, day))).toEqual(['b', 'c'])
    const both = { tags: ['winter', 'night'], logic: { tags: { combine: 'and' as const } } }
    expect(ids(narrowByColumns(rows, both, day))).toEqual(['a'])
    expect(ids(narrowByColumns(rows, { tags: [EMPTY_TICK] }, day))).toEqual(['c'])
    // A work is in one collection: "and" over two of them is read as "or".
    expect(andBlocked('collections', 2)).toBe(true)
    expect(andBlocked('tags', 2)).toBe(false)
  })

  it('reads a stored list naming the retired columns without them', () => {
    expect(sanitizeColumns(['title', 'tier', 'scored', 'ready', 'total'])).toEqual([
      'title',
      'total',
    ])
  })
})
