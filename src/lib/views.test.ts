import { describe, expect, it } from 'vitest'
import {
  addView,
  loadViews,
  matchesView,
  MAX_VIEWS,
  removeView,
  saveViews,
  viewOf,
  type SavedView,
} from './views'
import { DEFAULT_SORT, type SortStore } from './catalogue'

const view = (over: Partial<SavedView> = {}): SavedView => ({
  id: 'v1',
  name: 'Unscored clips',
  filter: { tier: 'clip', gap: 'unscored' },
  sort: DEFAULT_SORT,
  groupBy: 'none',
  ...over,
})

function store(seed: Record<string, string> = {}): SortStore & { held: Record<string, string> } {
  const held = { ...seed }
  return {
    held,
    getItem: (key) => held[key] ?? null,
    setItem: (key, value) => {
      held[key] = value
    },
  }
}

describe('loadViews', () => {
  it('starts with none', () => {
    expect(loadViews(store())).toEqual([])
  })

  it('reads back what was saved', () => {
    const held = store()
    saveViews([view()], held)

    expect(loadViews(held)).toEqual([view()])
  })

  it('drops one broken entry rather than the whole bar', () => {
    const held = store({
      'kilna.catalogue.views': JSON.stringify([view(), { id: 'x' }, view({ id: 'v2' })]),
    })

    expect(loadViews(held).map((held) => held.id)).toEqual(['v1', 'v2'])
  })

  it('survives a value that is not a list', () => {
    expect(loadViews(store({ 'kilna.catalogue.views': '{"id":"v1"}' }))).toEqual([])
  })

  it('survives text that is not JSON at all', () => {
    expect(loadViews(store({ 'kilna.catalogue.views': 'not json' }))).toEqual([])
  })

  it('refuses a view with no name, which nothing could label', () => {
    const held = store({ 'kilna.catalogue.views': JSON.stringify([view({ name: '  ' })]) })
    expect(loadViews(held)).toEqual([])
  })

  it('refuses a grouping this build does not draw', () => {
    const held = store({
      'kilna.catalogue.views': JSON.stringify([view({ groupBy: 'collection' as never })]),
    })
    expect(loadViews(held)).toEqual([])
  })

  it('does not break when storage throws', () => {
    const broken: SortStore = {
      getItem: () => {
        throw new Error('blocked')
      },
      setItem: () => {},
    }
    expect(loadViews(broken)).toEqual([])
  })
})

describe('saveViews', () => {
  it('does not break when storage is full', () => {
    const broken: SortStore = {
      getItem: () => null,
      setItem: () => {
        throw new Error('full')
      },
    }
    expect(() => saveViews([view()], broken)).not.toThrow()
  })
})

describe('addView', () => {
  it('appends a new name', () => {
    expect(addView([view()], view({ id: 'v2', name: 'Winter' }))).toHaveLength(2)
  })

  it('overwrites the view that already carries the name, keeping its id', () => {
    const next = addView([view()], view({ id: 'fresh', name: 'unscored CLIPS', groupBy: 'tier' }))

    expect(next).toHaveLength(1)
    expect(next[0]?.id).toBe('v1')
    expect(next[0]?.groupBy).toBe('tier')
  })

  it('stops at the reading limit', () => {
    const many = Array.from({ length: MAX_VIEWS }, (_, index) =>
      view({ id: `v${index}`, name: `View ${index}` }),
    )

    expect(addView(many, view({ id: 'extra', name: 'One more' }))).toHaveLength(MAX_VIEWS)
  })
})

describe('removeView', () => {
  it('takes one out by id', () => {
    const views = [view(), view({ id: 'v2', name: 'Winter' })]
    expect(removeView(views, 'v1').map((held) => held.id)).toEqual(['v2'])
  })
})

describe('matchesView', () => {
  it('is true for the arrangement it was saved from', () => {
    const held = view()
    expect(matchesView(held, { filter: held.filter, sort: held.sort, groupBy: held.groupBy })).toBe(
      true,
    )
  })

  it('comes off the moment a filter is touched', () => {
    const held = view()
    expect(
      matchesView(held, { filter: { tier: 'pic' }, sort: held.sort, groupBy: held.groupBy }),
    ).toBe(false)
  })

  it('comes off when the sort changes', () => {
    const held = view()
    expect(
      matchesView(held, {
        filter: held.filter,
        sort: { column: 'title', direction: 'asc' },
        groupBy: held.groupBy,
      }),
    ).toBe(false)
  })

  it('comes off when the grouping changes', () => {
    const held = view()
    expect(matchesView(held, { filter: held.filter, sort: held.sort, groupBy: 'tier' })).toBe(false)
  })

  // Focusing the box and clearing it leaves `search: ''`, which asks the same
  // question as no search at all; unhighlighting the view there would be a lie
  // about what changed.
  it('treats an empty search as no search', () => {
    const held = view({ filter: { tier: 'clip' } })

    expect(
      matchesView(held, {
        filter: { tier: 'clip', search: '' },
        sort: held.sort,
        groupBy: held.groupBy,
      }),
    ).toBe(true)
  })

  it('sees a tag that the saved view does not carry', () => {
    const held = view({ filter: {} })

    expect(
      matchesView(held, { filter: { tag: 'winter' }, sort: held.sort, groupBy: held.groupBy }),
    ).toBe(false)
  })
})

describe('viewOf', () => {
  it('trims the name and takes the arrangement as it stands', () => {
    const made = viewOf('  Winter  ', {
      filter: { tag: 'winter' },
      sort: DEFAULT_SORT,
      groupBy: 'tier',
    })

    expect(made.name).toBe('Winter')
    expect(made.filter).toEqual({ tag: 'winter' })
    expect(made.groupBy).toBe('tier')
    expect(made.id).not.toBe('')
  })
})
