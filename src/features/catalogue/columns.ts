import type { ColumnId, FilterableColumn, SortColumn } from '@/lib/catalogue'

/** How one column is headed, sized, aligned and narrowed. */
interface ColumnSpec {
  label: string
  /** The sort it drives, or null for a column that answers no ordering question. */
  sort: SortColumn | null
  /** The funnel it carries, or none. */
  filter?: FilterableColumn
  align?: 'left' | 'right'
  width?: string
  /** What the column is given under fixed layout when it was never measured
   * - turned on after the first drag. Close to what the browser would give
   * it, which is all it has to be: a drag corrects it. */
  naturalWidth: number
}

/**
 * Every column in one table, read by the header, the body and the picker.
 *
 * The header and the body used to be written out separately, and the fifth
 * column of a header could quietly describe the sixth cell of a row. Here a
 * column is one entry and cannot come apart.
 */
export const COLUMN_SPECS: Record<ColumnId, ColumnSpec> = {
  id: { label: 'catalogue.column.id', sort: null, width: 'w-28', naturalWidth: 112 },
  title: { label: 'catalogue.work', sort: 'title', filter: 'title', naturalWidth: 320 },
  marks: { label: 'catalogue.column.marks', sort: null, filter: 'marks', naturalWidth: 160 },
  status: {
    label: 'catalogue.column.status',
    sort: 'status',
    filter: 'statuses',
    naturalWidth: 120,
  },
  stage: {
    label: 'catalogue.column.stage',
    sort: 'stage',
    filter: 'stages',
    width: 'w-14',
    naturalWidth: 72,
  },
  tier: { label: 'catalogue.tier', sort: 'tier', filter: 'tiers', naturalWidth: 128 },
  total: { label: 'catalogue.total', sort: 'total', align: 'right', naturalWidth: 80 },
  versions: {
    label: 'catalogue.column.versions',
    sort: 'versions',
    align: 'right',
    width: 'w-16',
    naturalWidth: 64,
  },
  // Wider than a date: the heading is the longest word in the row, and a
  // column sized by its "Sep 22"s cut it to "Next releas".
  release: {
    label: 'catalogue.column.release',
    sort: 'release',
    width: 'w-36',
    naturalWidth: 144,
  },
  ready: { label: 'catalogue.column.ready', sort: null, width: 'w-28', naturalWidth: 112 },
  scored: { label: 'catalogue.scored', sort: 'scored', naturalWidth: 104 },
  created: { label: 'catalogue.column.created', sort: 'created', naturalWidth: 104 },
  updated: { label: 'catalogue.column.updated', sort: 'updated', naturalWidth: 104 },
}

/** The two cells that are not columns: the tick and the row menu. Sized once
 * the layout is fixed, to what their classes give them before. */
export const SELECT_WIDTH = 36
export const MENU_WIDTH = 40

/*
 * What stays while the table slides under it.
 *
 * The table scrolls sideways once there are more columns than window, and
 * everything travelled - so a row scrolled far enough to read its dates was a
 * row you could no longer name, tick or act on. The three that answer "which
 * work is this and what do I do with it" are held: the tick and the title
 * against the left edge, the row menu against the right.
 *
 * Each needs its own ground. A sticky cell is painted over by whatever slides
 * beneath it otherwise, and the rows would read through the title. The ground
 * is the panel's, `bg-raise`, and the row's tint is laid over it on a layer
 * of its own - a `::before` behind the cell's content - rather than as the
 * cell's background: the tints are translucent, and a held cell painted in
 * one alone let the columns sliding under it show through. The row draws its
 * own tint; a cell that paints itself reads the row's state through `group`
 * and lays the same tint on.
 *
 * The title keeps a floor under its width. It is draggable like any column,
 * and dragged to nothing it would hold a stripe of empty background against
 * the edge - the owner asked that some of the name always remain.
 */
const HELD =
  'sticky z-1 bg-raise ' +
  'before:pointer-events-none before:absolute before:inset-0 before:-z-1 ' +
  'group-hover:before:bg-soft group-data-[popup-open]:before:bg-soft ' +
  'group-aria-selected:before:bg-accent-soft'

export const STUCK_LEFT_TICK = `${HELD} left-0`
export const STUCK_LEFT_TITLE = HELD
export const STUCK_RIGHT_MENU = `${HELD} right-0`

/** A held heading: over the held cells at the corners, on the panel's ground. */
export const STUCK_HEADING = 'sticky z-2 bg-raise'

/**
 * Where the held title stands, for its heading and its cells alike: hard
 * against the tick, which never moves, and never narrower than the least of a
 * title that stays readable when its column is dragged in.
 */
export const TITLE_HOLD = { left: SELECT_WIDTH, minWidth: 180 } as const
