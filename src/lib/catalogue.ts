import type { ScheduledRelease, ScoredWork } from '@/lib/api/types'

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
  | 'release'
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
 * A work as the catalogue draws it: its row, and the release it is going out
 * with next.
 *
 * The catalogue's own payload knows how many releases a work has booked, not
 * which or when; the calendar's knows each booking, with the readiness the
 * backend judged for it. The two are joined here rather than a second
 * readiness being worked out on the way: the chip in the calendar and the
 * cell in this table then read the same verdict and cannot disagree.
 */
export interface CatalogueRow extends ScoredWork {
  /** The earliest booking that has not gone out yet, or null when nothing is
   * booked. A slot whose day has passed without the release being marked out
   * still counts: it is the booking most in need of an answer. */
  next_release: ScheduledRelease | null
}

/**
 * Give every row its next release, out of the calendar's list.
 *
 * Only what holds a day and has not gone out counts - the same two facts the
 * row's `scheduled` count is made of. A work's releases are compared by day,
 * then by the hour when one is set, so two on the same day resolve the same
 * way on every render. `releases` is absent while the calendar is still being
 * read, and every row then has none rather than waiting for it.
 */
export function withNextReleases(
  rows: readonly ScoredWork[],
  releases: readonly ScheduledRelease[] | undefined,
): CatalogueRow[] {
  const next = new Map<string, ScheduledRelease>()
  for (const release of releases ?? []) {
    if (release.scheduled_at === null || release.status === 'released') continue
    const held = next.get(release.work_id)
    if (held === undefined || sooner(release, held)) next.set(release.work_id, release)
  }
  // A song's next release is the soonest of what was made from it: it goes
  // out as its audio, its clip and its shorts (v0.86, ADR 0047), and its row
  // lists them in `publications`.
  return rows.map((row) => {
    let soonest = next.get(row.work_id) ?? null
    for (const made of row.publications ?? []) {
      const theirs = next.get(made)
      if (theirs !== undefined && (soonest === null || sooner(theirs, soonest))) soonest = theirs
    }
    return { ...row, next_release: soonest }
  })
}

/** Whether `a` goes out before `b`. Both hold a day; an hour is only a
 * tiebreak, and a release with none goes first, as the start of its day. */
function sooner(a: ScheduledRelease, b: ScheduledRelease): boolean {
  const day = (a.scheduled_at ?? '').localeCompare(b.scheduled_at ?? '')
  if (day !== 0) return day < 0
  const hour = (a.scheduled_time ?? '').localeCompare(b.scheduled_time ?? '')
  if (hour !== 0) return hour < 0
  return a.id.localeCompare(b.id) < 0
}

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
export function narrow<Row extends ScoredWork>(
  rows: Row[],
  filter: CatalogueFilter,
  matching?: readonly string[],
): Row[] {
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
 * How many works have each gap, across the whole catalogue.
 *
 * Of everything rather than of what is on show, as the kind chips count: a
 * chip reading 0 because another filter hid its works would say the gap is
 * closed when it is only out of sight. The count is what makes the chip
 * worth pressing - "Not scored · 12" is a job, "Not scored" is a label.
 */
export function countGaps(rows: readonly ScoredWork[]): Record<Gap, number> {
  const counts: Record<Gap, number> = { unscored: 0, unscheduled: 0, stale: 0 }
  for (const row of rows) for (const gap of GAPS) if (hasGap(row, gap)) counts[gap] += 1
  return counts
}

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
 * The ticked entry standing for "no stage at all".
 *
 * A stage is stored as the percentage of its stop, so the funnel could say
 * "standing at this stop" and had no way to say "standing nowhere" — and the
 * works that had never been given a stage were exactly the ones the owner
 * wanted to find. Negative, because every real stop is 0..=100, and a
 * sentinel that cannot collide with a value is the only kind worth having.
 * `narrow` already tells a stage of 0 from no stage; this lets the funnel do
 * the same.
 */
export const NO_STAGE = -1

/**
 * What the funnels in the column headers hold.
 *
 * A second filter beside `CatalogueFilter` rather than more fields on it: the
 * query box and the chips write that one as a single line, and a set of
 * ticked stages has no place in the line. The two compose as AND, and both
 * live for the session only, for the reason given at `loadFilter`.
 */
export interface ColumnFilters {
  /** Part of the title, matched case-insensitively. */
  title?: string
  /** Status keys: works standing at any of them. The toolbar had a dropdown
   * for one status until v0.79; the funnel on the column asks for several,
   * and `status:` in the box still asks for one. */
  statuses?: string[]
  /** Stage stops, as their percentages: works standing at any of them, plus
   * {@link NO_STAGE} for works standing at none. */
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

const FILTERABLE_COLUMNS: FilterableColumn[] = ['title', 'statuses', 'stages', 'tiers', 'marks']

/**
 * Narrow by the header funnels. Applied after `narrow`, on what it left.
 *
 * A work with no stage matches only {@link NO_STAGE}, the funnel's own entry
 * for standing nowhere; a work with no tier still matches none of the ticked
 * tiers, since a tier is earned rather than set and "not yet" is what the
 * empty cell already says. Marks match on any ticked key, not all — a person
 * ticking two marks is asking "which works carry either", and two ticked
 * statuses ask the same of where the works stand.
 */
export function narrowByColumns<Row extends ScoredWork>(
  rows: Row[],
  filters: ColumnFilters,
): Row[] {
  const needle = filters.title?.trim().toLowerCase() ?? ''
  const statuses = isColumnFiltered(filters, 'statuses') ? new Set(filters.statuses) : undefined
  const stages = isColumnFiltered(filters, 'stages') ? new Set(filters.stages) : undefined
  const tiers = isColumnFiltered(filters, 'tiers') ? new Set(filters.tiers) : undefined
  const marks = isColumnFiltered(filters, 'marks') ? new Set(filters.marks) : undefined

  return rows.filter((row) => {
    if (needle !== '' && !row.title.toLowerCase().includes(needle)) return false
    if (statuses !== undefined && !statuses.has(row.status)) return false
    if (stages !== undefined) {
      // A work with no stage matches only the "no stage" tick, and a work
      // with one matches only its own stop. Ticking both asks for either,
      // the way two ticked marks do.
      const matched = row.stage === null ? stages.has(NO_STAGE) : stages.has(row.stage)
      if (!matched) return false
    }
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

export function saveColumnFilters(filters: ColumnFilters, store: SortStore = sessionStorage): void {
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
  if (!isListOf(candidate.statuses, 'string')) return false
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
export function sortRows(rows: CatalogueRow[], sort: Sort): CatalogueRow[] {
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
function presence(a: CatalogueRow, b: CatalogueRow, column: SortColumn): number {
  const has = (row: CatalogueRow) => valueOf(row, column) !== null
  if (has(a) === has(b)) return 0
  return has(a) ? -1 : 1
}

function valueOf(row: CatalogueRow, column: SortColumn): string | number | null {
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
    case 'release':
      // Nothing booked is absent, not far off: it sorts last either way.
      return row.next_release?.scheduled_at ?? null
    case 'created':
      return row.created_at
    case 'updated':
      return row.updated_at
  }
}

function compare(a: CatalogueRow, b: CatalogueRow, column: SortColumn): number {
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
  'release',
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
 * order the profile lists it, which alphabetical at least keeps stable. The
 * next release is the one date asked about from the near end: what goes out
 * soonest is the question, and a year out is the answer to nothing.
 */
const ASCENDING_FIRST: SortColumn[] = ['title', 'status', 'release']

/** Every column the table can draw. */
export type ColumnId =
  | 'id'
  | 'title'
  | 'marks'
  | 'status'
  | 'stage'
  | 'tier'
  | 'total'
  | 'versions'
  | 'release'
  | 'ready'
  | 'scored'
  | 'created'
  | 'updated'

/** In the order the column picker lists the hidden ones: the mockup's order
 * for the columns it draws, then the dates. */
export const ALL_COLUMNS: ColumnId[] = [
  'id',
  'title',
  'marks',
  'status',
  'stage',
  'tier',
  'total',
  'versions',
  'release',
  'ready',
  'scored',
  'created',
  'updated',
]

/**
 * What the catalogue shows until someone says otherwise: the mockup's
 * columns, which read a row from what the work is to when it goes out and
 * whether it can.
 *
 * The identifier is off: it matters when talking to the assistant or to a
 * plugin, not when reading down a list of titles. The predecessor showed it
 * always and it earned its place in exactly one workflow. The dates are off
 * too - when a work was scored, added or edited is a sort, and the table
 * answers it by ordering rather than by a column of days.
 */
export const DEFAULT_COLUMNS: ColumnId[] = [
  'title',
  'marks',
  'status',
  'stage',
  'tier',
  'total',
  'versions',
  'release',
  'ready',
]

/** The title carries the row's identity and its link; it cannot be turned off. */
export const REQUIRED_COLUMN: ColumnId = 'title'

/** The columns v0.79 added to the default list. */
const INTRODUCED: readonly ColumnId[] = ['status', 'release', 'ready']

const COLUMNS_CHOSEN_KEY = 'kilna.catalogue.columnsChosen'

/**
 * A stored column list from before v0.79, shown with the columns it could not
 * have named, each after the column it follows in the default list.
 *
 * A profile that ever saved its columns keeps that list, and the list cannot
 * name Status, Next release and Ready - they did not exist. Left alone, the
 * columns the owner asked for would never appear for anyone who once chose
 * columns. They are shown rather than written: the catalogue wrote the
 * profile on opening once, and a write nobody made is a line in the history
 * and an undo step nobody asked for (removed in v0.77). The first choice the
 * person makes in the picker is stored as it is, and from then on `chosen`
 * says their list is theirs - a column they hid stays hidden.
 *
 * A list that already names any of the three was chosen knowing them.
 */
export function withIntroduced(list: ColumnId[], chosen: boolean): ColumnId[] {
  if (chosen || INTRODUCED.some((id) => list.includes(id))) return list
  const out = [...list]
  for (const id of INTRODUCED) {
    const before = DEFAULT_COLUMNS.slice(0, DEFAULT_COLUMNS.indexOf(id))
      .reverse()
      .find((other) => out.includes(other))
    out.splice(before === undefined ? out.length : out.indexOf(before) + 1, 0, id)
  }
  return out
}

/** Whether a column choice was made on this machine since v0.79. */
export function loadColumnsChosen(store: SortStore = localStorage): boolean {
  try {
    return store.getItem(COLUMNS_CHOSEN_KEY) === '1'
  } catch {
    return false
  }
}

export function saveColumnsChosen(store: SortStore = localStorage): void {
  try {
    store.setItem(COLUMNS_CHOSEN_KEY, '1')
  } catch {
    // Blocked storage: the new columns show again next time, which is the
    // lesser surprise than a choice that is lost.
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
 * The columns the catalogue opens with: the profile's, or the default when
 * the profile names none.
 *
 * The profile is the home of the list since v0.50 - a novel and a record are
 * read down different columns, so the choice belongs to the craft, not to the
 * machine. Until v0.77 a profile without columns was first given whatever
 * this machine had remembered from before; every workspace had made that
 * move, so the machine's copy is gone.
 */
export function columnsFor(profileColumns: string[] | null | undefined): ColumnId[] {
  return sanitizeColumns(profileColumns)
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
export function columnsForKind(home: ColumnHome, kind: string | undefined): ColumnId[] {
  const own = kind === undefined ? undefined : home.catalogue_columns_by_kind?.[kind]
  return Array.isArray(own) ? sanitizeColumns(own) : columnsFor(home.catalogue_columns)
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

  return current.includes(column) ? current.filter((id) => id !== column) : [...current, column]
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

export interface Group<Row extends ScoredWork = ScoredWork> {
  /** The value shared by the rows, or null for the block holding those without one. */
  key: string | null
  rows: Row[]
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
export function groupRows<Row extends ScoredWork>(rows: Row[], by: GroupBy): Group<Row>[] {
  if (by === 'none') return [{ key: null, rows }]

  const blocks = new Map<string, Row[]>()
  const without: Row[] = []

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

  const grouped: Group<Row>[] = [...blocks].map(([key, block]) => ({ key, rows: block }))
  if (without.length > 0) grouped.push({ key: null, rows: without })
  return grouped
}
