import type { ScoredWork } from '@/lib/api'

/** A working gap: something started but not carried through. */
export type Gap = 'unscored' | 'unscheduled' | 'stale'

export interface CatalogueFilter {
  search?: string
  status?: string
  kind?: string
  tier?: string
  /** One of the author's own words, matched whole and case-insensitively. */
  tag?: string
  gap?: Gap
  /** Only the starred: works marked to come back to. */
  bookmarked?: true
  /** A stage stop, as its percentage — works standing at exactly that stop.
      The percentage and not the key, because a row carries the number and the
      keys live in the profile: this module stays about rows. */
  stage?: number
}

/** A column the table can be ordered by. */
export type SortColumn =
  | 'title'
  | 'stage'
  | 'tier'
  | 'total'
  | 'scored'
  | 'status'
  | 'versions'
  | 'created'
  | 'updated'
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
 * The fields narrow here: the whole catalogue is one query and a few hundred
 * rows at most, so filtering costs nothing and the unfiltered total stays in
 * hand for a "showing N of M" count.
 *
 * The text does not, and cannot. A row holds a title, not the lyric under it,
 * so "which of my songs mention a fridge" is a question these rows have no way
 * to answer — it is asked of the index and arrives as `matching`, the ids the
 * search found, in the order it ranked them. `undefined` means nothing was
 * typed; an empty list means nothing matched, which is a different answer and
 * must narrow to nothing.
 *
 * The title is still matched here as well, so a half-typed title narrows on the
 * keystroke rather than on the round trip.
 */
export function narrow(
  rows: ScoredWork[],
  filter: CatalogueFilter,
  matching?: readonly string[],
): ScoredWork[] {
  const needle = filter.search?.trim().toLowerCase() ?? ''
  const found = matching === undefined ? undefined : new Set(matching)

  return rows.filter((row) => {
    if (filter.status !== undefined && row.status !== filter.status) return false
    if (filter.kind !== undefined && row.kind !== filter.kind) return false
    if (filter.tier !== undefined && row.tier !== filter.tier) return false
    // A whole word, not a substring: `tag:win` should not drag in `winter`.
    // Tags are chosen from what the workspace already holds, so the exact word
    // is the one a person means; substring matching would make the narrowest
    // tool in the box the vaguest.
    if (filter.tag !== undefined && !hasTag(row, filter.tag)) return false
    if (filter.gap !== undefined && !hasGap(row, filter.gap)) return false
    if (filter.bookmarked === true && row.bookmarked_at === null) return false
    if (filter.stage !== undefined && row.stage !== filter.stage) return false
    if (needle !== '') {
      // Either way of matching is enough: the index knows the bodies, and the
      // title check keeps a partly typed name narrowing before the search
      // answers. `холодильник` finds the lyric; `холод` finds the title.
      const byTitle = row.title.toLowerCase().includes(needle)
      const byText = found?.has(row.work_id) ?? false
      if (!byTitle && !byText) return false
    }
    return true
  })
}

/** Whether the work carries this tag, ignoring case in any language. */
function hasTag(row: ScoredWork, tag: string): boolean {
  const wanted = tag.toLowerCase()
  return row.tags.some((held) => held.toLowerCase() === wanted)
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

/** True when anything is narrowing the list — what an empty result has to
 * explain. The header funnels count too, when they are handed over. */
export function isNarrowed(filter: CatalogueFilter, columns?: ColumnFilters): boolean {
  return (
    (filter.search !== undefined && filter.search.trim() !== '') ||
    filter.status !== undefined ||
    filter.kind !== undefined ||
    filter.tier !== undefined ||
    filter.tag !== undefined ||
    filter.gap !== undefined ||
    filter.bookmarked === true ||
    // Left out when the stage filter arrived, so a catalogue narrowed to one
    // stop showed no count and no "clear" — and an empty result blamed nothing.
    filter.stage !== undefined ||
    isNarrowedByColumns(columns)
  )
}

/**
 * What the funnels in the column headers hold.
 *
 * A second filter beside `CatalogueFilter` rather than more fields on it: the
 * query box and the dropdowns write that one as a single line, and a set of
 * ticked stages has no place in the line. The two compose as AND, and both
 * live for the session only, for the reason given at `loadFilter`.
 */
export interface ColumnFilters {
  /** Part of the title, matched case-insensitively. */
  title?: string
  /** Stage stops, as their percentages: works standing at any of them. */
  stages?: number[]
  /** Tier keys. */
  tiers?: string[]
  /** Mark keys: works carrying any of them. */
  marks?: string[]
}

/** The column filters, as the header funnels tell them apart. */
export type FilterableColumn = keyof ColumnFilters

/** Whether one funnel is holding anything. An empty list and an empty string
 * both mean "no filter", so a person who unticks the last box is back to
 * everything without having to find a clear button. */
export function isColumnFiltered(filters: ColumnFilters, column: FilterableColumn): boolean {
  const held = filters[column]
  if (held === undefined) return false
  return typeof held === 'string' ? held.trim() !== '' : held.length > 0
}

/** True when any header funnel is narrowing the list. */
export function isNarrowedByColumns(filters: ColumnFilters = {}): boolean {
  return FILTERABLE_COLUMNS.some((column) => isColumnFiltered(filters, column))
}

export const FILTERABLE_COLUMNS: FilterableColumn[] = ['title', 'stages', 'tiers', 'marks']

/**
 * Narrow by the header funnels. Applied after `narrow`, on what it left.
 *
 * Absence follows `narrow`: a work with no stage stands at no stop, so it
 * matches none of the ticked ones; the same for a work with no tier. Marks
 * match on any ticked key, not all — a person ticking two marks is asking
 * "which works carry either", the way a status dropdown asks about one.
 */
export function narrowByColumns(rows: ScoredWork[], filters: ColumnFilters): ScoredWork[] {
  const needle = filters.title?.trim().toLowerCase() ?? ''
  const stages = isColumnFiltered(filters, 'stages') ? new Set(filters.stages) : undefined
  const tiers = isColumnFiltered(filters, 'tiers') ? new Set(filters.tiers) : undefined
  const marks = isColumnFiltered(filters, 'marks') ? new Set(filters.marks) : undefined

  return rows.filter((row) => {
    if (needle !== '' && !row.title.toLowerCase().includes(needle)) return false
    if (stages !== undefined && (row.stage === null || !stages.has(row.stage))) return false
    if (tiers !== undefined && (row.tier === null || !tiers.has(row.tier))) return false
    if (marks !== undefined && !row.marks.some((key) => marks.has(key))) return false
    return true
  })
}

const COLUMN_FILTERS_KEY = 'kilna.catalogue.columnFilters'

/** The column filters have the lifetime of the query filter, for the same
 * reason: a funnel still narrowing the table tomorrow reads as lost data. */
export function loadColumnFilters(store: SortStore = sessionStorage): ColumnFilters {
  try {
    const raw = store.getItem(COLUMN_FILTERS_KEY)
    if (raw === null) return {}

    const parsed: unknown = JSON.parse(raw)
    return isColumnFilters(parsed) ? parsed : {}
  } catch {
    return {}
  }
}

export function saveColumnFilters(
  filters: ColumnFilters,
  store: SortStore = sessionStorage,
): void {
  try {
    store.setItem(COLUMN_FILTERS_KEY, JSON.stringify(filters))
  } catch {
    // Storage full or blocked: the funnels still narrow, they just forget.
  }
}

/** Only the shape is checked, as with `isFilter`: a stage or a tier the
 * profile no longer has narrows to nothing, which the count explains. */
function isColumnFilters(value: unknown): value is ColumnFilters {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) return false
  const candidate = value as Record<string, unknown>

  if (candidate.title !== undefined && typeof candidate.title !== 'string') return false
  if (!isListOf(candidate.stages, 'number')) return false
  if (!isListOf(candidate.tiers, 'string')) return false
  return isListOf(candidate.marks, 'string')
}

function isListOf(value: unknown, type: 'string' | 'number'): boolean {
  if (value === undefined) return true
  return Array.isArray(value) && value.every((item) => typeof item === type)
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
    case 'status':
      return row.status
    case 'versions':
      return row.version_count
    case 'stage':
      // Unjudged is not zero, so it sorts as absent and lands last with
      // everything else nobody has answered for.
      return row.stage
    case 'created':
      return row.created_at
    case 'updated':
      return row.updated_at
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

const COLUMNS: SortColumn[] = [
  'title',
  // Missing from this list when the stage sort arrived, so a stage sort was
  // remembered and then refused on the next open, quietly resetting to the
  // score. The list is what `valueOf` switches on, and the two must agree.
  'stage',
  'tier',
  'total',
  'scored',
  'status',
  'versions',
  'created',
  'updated',
]

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

  for (const field of ['search', 'status', 'kind', 'tier', 'tag'] as const) {
    const held = candidate[field]
    if (held !== undefined && typeof held !== 'string') return false
  }

  if (candidate.bookmarked !== undefined && candidate.bookmarked !== true) return false

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
  return { column, direction: ASCENDING_FIRST.includes(column) ? 'asc' : 'desc' }
}

/**
 * Columns whose first click should read forwards.
 *
 * A number or a date is nearly always asked about from the top — the best score,
 * the most recent edit. Words are asked about from A, and a status reads in the
 * order the profile lists it, which alphabetical at least keeps stable.
 */
const ASCENDING_FIRST: SortColumn[] = ['title', 'status']

/** Every column the table can draw, in the order it draws them. */
export type ColumnId =
  | 'id'
  | 'title'
  | 'stage'
  | 'marks'
  | 'versions'
  | 'tier'
  | 'total'
  | 'scored'
  | 'created'
  | 'updated'

export const ALL_COLUMNS: ColumnId[] = [
  'id',
  'title',
  'stage',
  'marks',
  'versions',
  'tier',
  'total',
  'scored',
  'created',
  'updated',
]

/**
 * What the catalogue shows until someone says otherwise.
 *
 * The identifier is off: it matters when talking to the assistant or to a
 * plugin, not when reading down a list of titles. The predecessor showed it
 * always and it earned its place in exactly one workflow.
 */
export const DEFAULT_COLUMNS: ColumnId[] = [
  'title',
  'stage',
  'marks',
  'tier',
  'total',
  'scored',
  'updated',
]

/** The title carries the row's identity and its link; it cannot be turned off. */
export const REQUIRED_COLUMN: ColumnId = 'title'

const COLUMNS_KEY = 'kilna.catalogue.columns'

/**
 * Which columns are shown survives a restart, like the sort and unlike the
 * filter: it is how a person prefers to read the table, not what they are doing
 * this minute.
 *
 * Kept in the browser rather than on the profile. The plan asked for the latter,
 * but a field on the profile means a migration, and the release this belongs to
 * gathers every schema change into one — see the Model package. Moving it there
 * later is a read of this key and a write of the new field.
 */
export function loadColumns(store: SortStore = localStorage): ColumnId[] {
  try {
    const raw = store.getItem(COLUMNS_KEY)
    if (raw === null) return DEFAULT_COLUMNS
    return sanitizeColumns(JSON.parse(raw))
  } catch {
    return DEFAULT_COLUMNS
  }
}

/**
 * A stored list of column ids, made safe to draw.
 *
 * The order is kept as stored: it is the order the table draws, and the one
 * the person made. Unknown ids are dropped rather than rejected wholesale: a
 * column removed in a later build should not cost the rest of the layout. Everything
 * hidden is indistinguishable from a corrupt value, and an empty table
 * teaches nobody anything, so that falls back to the default.
 */
export function sanitizeColumns(parsed: unknown): ColumnId[] {
  if (!Array.isArray(parsed)) return DEFAULT_COLUMNS
  const known = parsed.filter((id): id is ColumnId => ALL_COLUMNS.includes(id as ColumnId))
  const shown = known.includes(REQUIRED_COLUMN) ? known : [REQUIRED_COLUMN, ...known]
  return shown.length > 1 || known.includes(REQUIRED_COLUMN) ? shown : DEFAULT_COLUMNS
}

/**
 * The columns the catalogue opens with: the profile's, and until the profile
 * has any, whatever this machine remembered.
 *
 * The profile is the home of the list since v0.50 — a novel and a record are
 * read down different columns, so the choice belongs to the craft, not to the
 * browser. The browser's copy is what every workspace holds from before, and
 * it is read exactly once: the first time a profile without columns is
 * opened, so the move costs nobody their layout. `fromMachine` says that is
 * what happened, so the caller can write the list to the profile and be done
 * with the key.
 */
export function columnsFor(
  profileColumns: string[] | null | undefined,
  store: SortStore = localStorage,
): { columns: ColumnId[]; fromMachine: boolean } {
  if (Array.isArray(profileColumns)) {
    return { columns: sanitizeColumns(profileColumns), fromMachine: false }
  }
  return { columns: loadColumns(store), fromMachine: true }
}

/** The slice of a profile the column choice reads and writes. */
export interface ColumnHome {
  catalogue_columns?: string[] | null
  catalogue_columns_by_kind?: Record<string, string[]> | null
}

/**
 * The columns for the table as it is narrowed: a kind's own list when the
 * profile keeps one for it, the profile's list otherwise.
 *
 * A video is read down other columns than a song, and the catalogue narrowed
 * to videos is the moment that shows. A kind that was never given columns of
 * its own reads down the shared list rather than the default, so narrowing
 * never costs a person the layout they already chose.
 */
export function columnsForKind(
  home: ColumnHome,
  kind: string | undefined,
  store: SortStore = localStorage,
): { columns: ColumnId[]; fromMachine: boolean } {
  const own = kind === undefined ? undefined : home.catalogue_columns_by_kind?.[kind]
  if (Array.isArray(own)) return { columns: sanitizeColumns(own), fromMachine: false }
  return columnsFor(home.catalogue_columns, store)
}

/**
 * The profile fields to write so that `columnsForKind` answers `next` for
 * this kind — and only for it: choosing columns while narrowed to videos
 * must not change how songs are read.
 */
export function withColumns(
  home: ColumnHome,
  kind: string | undefined,
  next: ColumnId[],
): ColumnHome {
  if (kind === undefined) return { catalogue_columns: next }
  return {
    catalogue_columns_by_kind: { ...(home.catalogue_columns_by_kind ?? {}), [kind]: next },
  }
}

export function saveColumns(columns: ColumnId[], store: SortStore = localStorage): void {
  try {
    store.setItem(COLUMNS_KEY, JSON.stringify(columns))
  } catch {
    // Storage full or blocked: the table still draws, it just forgets.
  }
}

/**
 * Turn a column on or off.
 *
 * The stored order is the drawn order, since columns can be put in an order
 * of one's own: a column turned back on goes to the end, where it is found
 * without looking, and is dragged from there to where it belongs. Until v0.74
 * the list was re-sorted into `ALL_COLUMNS` on every toggle, which would
 * undo any order a person had made.
 */
export function toggleColumn(current: ColumnId[], column: ColumnId): ColumnId[] {
  if (column === REQUIRED_COLUMN) return current

  return current.includes(column)
    ? current.filter((id) => id !== column)
    : [...current, column]
}

/**
 * Put a column at a position, counted in the list it ends up in.
 *
 * `to` is clamped rather than refused: a keyboard nudging the first column
 * further up means "leave it first", not "do nothing and beep". A column the
 * list does not hold cannot be moved and the list comes back untouched.
 */
export function moveColumn(current: ColumnId[], column: ColumnId, to: number): ColumnId[] {
  const from = current.indexOf(column)
  if (from === -1) return current

  const target = Math.max(0, Math.min(current.length - 1, to))
  if (target === from) return current

  const without = current.filter((id) => id !== column)
  return [...without.slice(0, target), column, ...without.slice(target)]
}

/** The per-machine widths a person dragged, by column, in pixels. */
export type ColumnWidths = Partial<Record<ColumnId, number>>

const WIDTHS_KEY = 'kilna.catalogue.widths'

/** Narrower than this and a column is a stripe with nothing readable in it. */
export const MIN_COLUMN_WIDTH = 56

/**
 * The widths survive a restart and stay on this machine.
 *
 * They are not on the profile beside the columns, because a width is a fact
 * about a screen: the pixels that fit a title on a laptop are not the pixels
 * that fit it on a monitor, and following the profile to a second machine
 * would carry the wrong answer there. The sort draws the same line.
 */
export function loadWidths(store: SortStore = localStorage): ColumnWidths {
  try {
    const raw = store.getItem(WIDTHS_KEY)
    if (raw === null) return {}
    return sanitizeWidths(JSON.parse(raw))
  } catch {
    return {}
  }
}

export function saveWidths(widths: ColumnWidths, store: SortStore = localStorage): void {
  try {
    store.setItem(WIDTHS_KEY, JSON.stringify(widths))
  } catch {
    // Storage full or blocked: the table still draws, it just forgets.
  }
}

/** Unknown columns and unusable numbers are dropped one by one, as with the
 * column list: a column removed in a later build should not cost the rest. */
function sanitizeWidths(parsed: unknown): ColumnWidths {
  if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) return {}
  const widths: ColumnWidths = {}
  for (const [id, value] of Object.entries(parsed)) {
    if (!ALL_COLUMNS.includes(id as ColumnId)) continue
    if (typeof value !== 'number' || !Number.isFinite(value)) continue
    widths[id as ColumnId] = Math.max(MIN_COLUMN_WIDTH, Math.round(value))
  }
  return widths
}

/** How the rows are gathered into blocks. */
export type GroupBy = 'none' | 'status' | 'tier'

export interface Group {
  /** The value shared by the rows, or null for the block holding those without one. */
  key: string | null
  rows: ScoredWork[]
}

/**
 * Gather the rows into blocks, keeping the sort inside each.
 *
 * Grouping by collection is deliberately absent. The column exists, but the
 * screen that gives collections meaning does not yet, and offering to group by
 * something a person cannot yet see or edit promises a feature twice.
 *
 * Blocks come out in the order the rows already had, so the sort still decides
 * which block leads — a catalogue grouped by tier and sorted by score opens on
 * the tier holding the best work. Rows with no value form a block of their own,
 * last, for the same reason absence sorts last within a column.
 */
export function groupRows(rows: ScoredWork[], by: GroupBy): Group[] {
  if (by === 'none') return [{ key: null, rows }]

  const blocks = new Map<string, ScoredWork[]>()
  const without: ScoredWork[] = []

  for (const row of rows) {
    const key = by === 'status' ? row.status : row.tier
    if (key === null || key === '') {
      without.push(row)
      continue
    }

    const block = blocks.get(key)
    if (block) block.push(row)
    else blocks.set(key, [row])
  }

  const grouped: Group[] = [...blocks].map(([key, block]) => ({ key, rows: block }))
  if (without.length > 0) grouped.push({ key: null, rows: without })
  return grouped
}
