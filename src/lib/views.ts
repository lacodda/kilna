/**
 * A slice of the catalogue, kept under a name.
 *
 * "Unscored clips", "winter songs still in draft" — the questions a person asks
 * the catalogue every week. Setting three dropdowns and a grouping every time
 * is the kind of friction that ends with nobody narrowing anything, so the
 * whole arrangement gets a name and one click.
 *
 * ## What a view holds, and what it deliberately does not
 *
 * Filter, sort and grouping: everything that decides *which rows and in what
 * shape*. Not the chosen columns, which are how a person reads the table
 * everywhere rather than a property of one question, and not the selection,
 * which is about the click being made this second.
 *
 * They are drawn on the catalogue itself rather than in the app's left rail:
 * the rail lists the screens, and a slice of the catalogue named there while
 * the calendar is open would point at something not on show.
 *
 * ## Why `localStorage` and not the profile
 *
 * A view is a standing choice, like the sort and the columns, so it outlives a
 * restart. Putting it on the profile means a migration and a format version,
 * and the release that gathers those is the Model package — see the plan.
 * Moving it there later reads this key and writes the new field, the same path
 * the columns already have marked out for them.
 */
import type { CatalogueFilter, GroupBy, Sort, SortStore } from '@/lib/catalogue'

export interface SavedView {
  /** Stable across renames, so the active view survives being retitled. */
  id: string
  name: string
  filter: CatalogueFilter
  sort: Sort
  groupBy: GroupBy
}

/** What a view is made of, before it has a name and an identity. */
export type ViewShape = Pick<SavedView, 'filter' | 'sort' | 'groupBy'>

const VIEWS_KEY = 'kilna.catalogue.views'

/**
 * How many views the bar will hold.
 *
 * Not a storage limit — a reading one. Past a dozen the bar stops being a short
 * row of standing questions and becomes a second catalogue to search through,
 * which is the thing views exist to avoid.
 */
export const MAX_VIEWS = 12

export function loadViews(store: SortStore = localStorage): SavedView[] {
  try {
    const raw = store.getItem(VIEWS_KEY)
    if (raw === null) return []

    const parsed: unknown = JSON.parse(raw)
    if (!Array.isArray(parsed)) return []

    // One bad entry costs its own view, not the whole bar.
    return parsed.filter(isView).slice(0, MAX_VIEWS)
  } catch {
    return []
  }
}

export function saveViews(views: SavedView[], store: SortStore = localStorage): void {
  try {
    store.setItem(VIEWS_KEY, JSON.stringify(views))
  } catch {
    // Storage full or blocked: the catalogue still narrows, it just cannot
    // remember how.
  }
}

function isView(value: unknown): value is SavedView {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) return false
  const candidate = value as Record<string, unknown>

  if (typeof candidate.id !== 'string' || typeof candidate.name !== 'string') return false
  if (candidate.name.trim() === '') return false
  if (typeof candidate.filter !== 'object' || candidate.filter === null) return false
  if (Array.isArray(candidate.filter)) return false

  const groupBy = candidate.groupBy
  if (groupBy !== 'none' && groupBy !== 'status' && groupBy !== 'tier') return false

  // The sort is checked loosely on purpose: `sortRows` falls back on an unknown
  // column rather than throwing, and a view is worth keeping for its filter
  // even if a later build dropped the column it sorted by.
  const sort = candidate.sort
  if (typeof sort !== 'object' || sort === null) return false
  return true
}

/**
 * Add a view, or overwrite the one that already carries the name.
 *
 * Saving twice under one name is how a person edits a view — set it up, adjust
 * it, save it again. A second "Unscored clips" beside the first would be a
 * mistake nobody meant to make, so the name is the identity for saving even
 * though the id is the identity for everything else.
 */
export function addView(views: SavedView[], view: SavedView): SavedView[] {
  const wanted = view.name.trim().toLowerCase()
  const existing = views.findIndex((held) => held.name.trim().toLowerCase() === wanted)
  const replaced = views[existing]

  if (replaced !== undefined) {
    const next = [...views]
    // The id of the view being replaced is kept, so anything pointing at it
    // still points at it after the overwrite.
    next[existing] = { ...view, id: replaced.id }
    return next
  }

  return [...views, view].slice(0, MAX_VIEWS)
}

export function removeView(views: SavedView[], id: string): SavedView[] {
  return views.filter((view) => view.id !== id)
}

/**
 * Whether what is on screen is still the view that was opened.
 *
 * The bar highlights the active view, and the highlight has to come off the
 * moment a dropdown is touched: showing "Unscored clips" as active while the
 * table shows something else is a lie about what is being looked at.
 */
export function matchesView(view: SavedView, shape: ViewShape): boolean {
  return (
    view.groupBy === shape.groupBy &&
    view.sort.column === shape.sort.column &&
    view.sort.direction === shape.sort.direction &&
    sameFilter(view.filter, shape.filter)
  )
}

const FILTER_FIELDS = ['search', 'status', 'kind', 'tier', 'tag', 'gap'] as const

function sameFilter(a: CatalogueFilter, b: CatalogueFilter): boolean {
  // An empty search and an absent one are the same question. Comparing the raw
  // objects would call them different and unhighlight a view the moment the box
  // was focused and cleared.
  return FILTER_FIELDS.every((field) => (a[field] ?? '') === (b[field] ?? ''))
}

/** A fresh view of what is currently on screen. */
export function viewOf(name: string, shape: ViewShape): SavedView {
  return {
    id: crypto.randomUUID(),
    name: name.trim(),
    filter: shape.filter,
    sort: shape.sort,
    groupBy: shape.groupBy,
  }
}
