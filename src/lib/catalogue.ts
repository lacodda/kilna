import type { ScoredWork } from '@/lib/api'

/** A working gap: something started but not carried through. */
export type Gap = 'unscored' | 'unscheduled' | 'stale'

export interface CatalogueFilter {
  search?: string
  status?: string
  kind?: string
  tier?: string
  gap?: Gap
}

/** A column the table can be ordered by. */
export type SortColumn = 'title' | 'tier' | 'total' | 'scored'
export type SortDirection = 'asc' | 'desc'

export interface Sort {
  column: SortColumn
  direction: SortDirection
}

/** What the catalogue opens on: the strongest work first, as it always has. */
export const DEFAULT_SORT: Sort = { column: 'total', direction: 'desc' }

/**
 * Narrow the catalogue in the app rather than in SQL.
 *
 * The whole catalogue is one query and a few hundred rows at most, so filtering
 * here costs nothing and buys two things the database cannot give as cheaply:
 * the unfiltered total is still in hand for a "showing N of M" count, and the
 * case folding is JavaScript's, which handles Cyrillic — SQLite's `LIKE` folds
 * ASCII only, the defect that cost v0.19 a fix in two places.
 */
export function narrow(rows: ScoredWork[], filter: CatalogueFilter): ScoredWork[] {
  const needle = filter.search?.trim().toLowerCase() ?? ''

  return rows.filter((row) => {
    if (filter.status !== undefined && row.status !== filter.status) return false
    if (filter.kind !== undefined && row.kind !== filter.kind) return false
    if (filter.tier !== undefined && row.tier !== filter.tier) return false
    if (filter.gap !== undefined && !hasGap(row, filter.gap)) return false
    if (needle !== '' && !row.title.toLowerCase().includes(needle)) return false
    return true
  })
}

/** The gaps a work can have, in the order they are offered. Exported because
 * the chips above the table and the check on a stored filter must be reading
 * one list - two would agree today and differ after the next gap is added. */
export const GAPS: Gap[] = ['unscored', 'unscheduled', 'stale']

/**
 * Whether a work is missing a step it has already earned.
 *
 * Only what can still be acted on counts. The predecessor's first version of
 * this listed ten works that had already gone out, offering to fix something
 * finished — the rule since: the automation shows what is still open.
 */
function hasGap(row: ScoredWork, gap: Gap): boolean {
  switch (gap) {
    // Nothing has judged it yet, so nothing can rank it.
    case 'unscored':
      return row.total === null
    // Judged, nothing out, nothing booked — the work that is ready and waiting.
    // A released work is deliberately excluded: it needs nothing.
    case 'unscheduled':
      return row.total !== null && row.released === 0 && row.scheduled === 0
    // The score describes a draft that has since been rewritten.
    case 'stale':
      return row.stale
  }
}

/** True when anything is narrowing the list — what an empty result has to explain. */
export function isNarrowed(filter: CatalogueFilter): boolean {
  return (
    (filter.search !== undefined && filter.search.trim() !== '') ||
    filter.status !== undefined ||
    filter.kind !== undefined ||
    filter.tier !== undefined ||
    filter.gap !== undefined
  )
}

/**
 * Order the rows.
 *
 * Unscored works sort last whichever way the score column points: they are not
 * the worst, they are unjudged, and burying the ranked ones under them would
 * make "worst first" useless. Sorting is stable on the title, so a column of
 * ties keeps a predictable order instead of shuffling between renders.
 */
export function sortRows(rows: ScoredWork[], sort: Sort): ScoredWork[] {
  const sign = sort.direction === 'asc' ? 1 : -1

  return [...rows].sort((a, b) => {
    // Absence is settled before the direction is applied. Ranking it with the
    // rest and flipping the sign would float unjudged works to the top of a
    // descending sort — measured, not assumed: it is what the first version of
    // this did.
    const missing = presence(a, b, sort.column)
    if (missing !== 0) return missing

    const ranked = compare(a, b, sort.column)
    if (ranked !== 0) return ranked * sign
    return a.title.localeCompare(b.title)
  })
}

/** Which of the two lacks a value in this column; absent sorts last, always. */
function presence(a: ScoredWork, b: ScoredWork, column: SortColumn): number {
  const has = (row: ScoredWork) => valueOf(row, column) !== null
  if (has(a) === has(b)) return 0
  return has(a) ? -1 : 1
}

function valueOf(row: ScoredWork, column: SortColumn): string | number | null {
  switch (column) {
    case 'title':
      return row.title
    case 'tier':
      return row.tier
    case 'total':
      return row.total
    case 'scored':
      return row.scored_at
  }
}

function compare(a: ScoredWork, b: ScoredWork, column: SortColumn): number {
  const left = valueOf(a, column)
  const right = valueOf(b, column)
  if (left === null || right === null) return 0

  return typeof left === 'number' && typeof right === 'number'
    ? left - right
    : String(left).localeCompare(String(right))
}

const SORT_KEY = 'kilna.catalogue.sort'

/**
 * Just enough of `localStorage` to be handed a fake one.
 *
 * The test runner has no DOM on purpose — component tests belong with the wider
 * testing pass in the 0.41 block — so rather than pull in `jsdom` for two
 * functions, the storage is a parameter and the browser's is the default.
 */
export interface SortStore {
  getItem: (key: string) => string | null
  setItem: (key: string, value: string) => void
}

/**
 * The sort survives a restart; the filters deliberately do not.
 *
 * Sorting is how a person prefers to read the table — a standing choice. A
 * filter is about the next thing they are doing, and finding the catalogue
 * still hiding most of it a day later reads as data loss rather than a setting.
 * The predecessor drew the same line for the same reason.
 */
export function loadSort(store: SortStore = localStorage): Sort {
  try {
    const raw = store.getItem(SORT_KEY)
    if (raw === null) return DEFAULT_SORT

    const parsed: unknown = JSON.parse(raw)
    return isSort(parsed) ? parsed : DEFAULT_SORT
  } catch {
    // A corrupt or unreadable value is not worth a broken screen.
    return DEFAULT_SORT
  }
}

export function saveSort(sort: Sort, store: SortStore = localStorage): void {
  try {
    store.setItem(SORT_KEY, JSON.stringify(sort))
  } catch {
    // Storage full or blocked: the table still sorts, it just forgets.
  }
}

const COLUMNS: SortColumn[] = ['title', 'tier', 'total', 'scored']

/**
 * Whether a stored value still describes a sort this build understands.
 *
 * A column removed in a later version would otherwise come back from storage
 * and reach `valueOf`, which has no case for it.
 */
function isSort(value: unknown): value is Sort {
  if (typeof value !== 'object' || value === null) return false
  const candidate = value as Partial<Sort>
  return (
    COLUMNS.includes(candidate.column as SortColumn) &&
    (candidate.direction === 'asc' || candidate.direction === 'desc')
  )
}

const FILTER_KEY = 'kilna.catalogue.filter'

/**
 * The filter holds for as long as the app is open, and no longer.
 *
 * Two failures sit on either side of this, and the middle is the only place
 * without one. Losing the filter on every visit to a card and back makes the
 * catalogue hostile to the way it is actually used - open a work, come back,
 * open the next. Keeping it across restarts is the other failure, and the one
 * the predecessor made: finding the catalogue still hiding most of the
 * library a day later reads as data loss rather than as a setting.
 *
 * So it lives in `sessionStorage` rather than `localStorage` - the same shape
 * as the sort, a different lifetime. Closing the window is the reset, and it
 * is one nobody has to remember to perform.
 */
export function loadFilter(store: SortStore = sessionStorage): CatalogueFilter {
  try {
    const raw = store.getItem(FILTER_KEY)
    if (raw === null) return {}

    const parsed: unknown = JSON.parse(raw)
    return isFilter(parsed) ? parsed : {}
  } catch {
    // A corrupt value is not worth a broken screen; an empty filter shows
    // everything, which is the safe way to be wrong.
    return {}
  }
}

export function saveFilter(filter: CatalogueFilter, store: SortStore = sessionStorage): void {
  try {
    store.setItem(FILTER_KEY, JSON.stringify(filter))
  } catch {
    // Storage full or blocked: the table still filters, it just forgets.
  }
}

/**
 * Whether a stored value still describes a filter this build understands.
 *
 * Only the shape is checked, not the values: a status or a kind comes from the
 * profile and can legitimately be anything, and a filter naming one that no
 * longer exists narrows to nothing rather than breaking - which the "showing
 * none of M" state already explains. A `gap` is different, because `hasGap`
 * switches on it exhaustively and an unknown one would fall through.
 */
function isFilter(value: unknown): value is CatalogueFilter {
  // An array is `typeof 'object'` too, and every field of one reads as
  // `undefined` - so `[1, 2]` passed every check below and came back as a
  // filter. Found by mutation testing the guard above it.
  if (typeof value !== 'object' || value === null || Array.isArray(value)) return false
  const candidate = value as Record<string, unknown>

  for (const field of ['search', 'status', 'kind', 'tier'] as const) {
    const held = candidate[field]
    if (held !== undefined && typeof held !== 'string') return false
  }

  const gap = candidate.gap
  return gap === undefined || GAPS.includes(gap as Gap)
}

/**
 * What clicking a column header does.
 *
 * The same column flips direction; a different one starts at the direction that
 * answers the question being asked — highest score first, but titles from A.
 */
export function toggleSort(current: Sort, column: SortColumn): Sort {
  if (current.column === column) {
    return { column, direction: current.direction === 'asc' ? 'desc' : 'asc' }
  }
  return { column, direction: column === 'title' ? 'asc' : 'desc' }
}
