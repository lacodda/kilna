import { useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { ChevronRight } from 'lucide-react'
import {
  groupRows,
  loadWidths,
  MIN_COLUMN_WIDTH,
  saveWidths,
  type CatalogueRow,
  type ColumnFilters,
  type ColumnId,
  type GroupBy,
  type Sort,
  type SortColumn,
} from '@/lib/catalogue'
import { allOf, labelOf, useProfile } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import type { Tab } from '@/features/work/tabs'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { measureColumns, useColumnWidths } from '@/components/ui/column-resize-handle'
import { EmptyState } from '@/components/ui/empty-state'
import { RowButton } from '@/components/ui/list-row'
import { Panel } from '@/components/ui/panel'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Table, TableBody, TableCell, TableHead, TableHeader } from '@/components/ui/table'
import { RowMenu, type RowAction } from '@/components/RowMenu'
import { CatalogueCell } from './CatalogueCell'
import { ColumnFunnel } from './ColumnFunnel'
import { ColumnHeader } from './ColumnHeader'
import {
  COLUMN_SPECS,
  MENU_WIDTH,
  SELECT_WIDTH,
  STUCK_HEADING,
  STUCK_LEFT_TICK,
  STUCK_RIGHT_MENU,
} from './columns'
import { WorkRow } from './WorkRow'

interface Props {
  /** What is on show: narrowed and sorted. */
  rows: CatalogueRow[]
  columns: ColumnId[]
  sort: Sort
  onReorder: (column: SortColumn) => void
  columnFilters: ColumnFilters
  onColumnFilters: (next: ColumnFilters) => void
  groupBy: GroupBy
  collapsed: ReadonlySet<string>
  onToggleGroup: (key: string) => void
  /** The table shows one kind of work: naming it on every row says nothing. */
  kindNarrowed: boolean
  selected: ReadonlySet<string>
  onSelectionChange: (next: ReadonlySet<string>) => void
  onSelect: (workId: string, tab?: Tab) => void
  onDelete: (workIds: readonly string[]) => void
  onClearFilters: () => void
}

/**
 * The works, in a panel of their own.
 *
 * The panel holds the height the screen leaves it and scrolls inside, both
 * ways, on the line's overlay bar. Narrow, every column used to squeeze until
 * dates broke across two lines and a row stood three lines tall, and the last
 * columns and the row menu were simply beyond the edge; now the table is as
 * wide as its columns and slides sideways under the three cells that are held.
 * The sideways bar sits at the foot of the panel, where it can be reached from
 * the middle of two hundred rows, and the headings stay while the rows move
 * under them - the scroller is the table's own, so a sticky heading has
 * something to stick to.
 */
export function CatalogueTable({
  rows,
  columns,
  sort,
  onReorder,
  columnFilters,
  onColumnFilters,
  groupBy,
  collapsed,
  onToggleGroup,
  kindNarrowed,
  selected,
  onSelectionChange,
  onSelect,
  onDelete,
  onClearFilters,
}: Props) {
  const { t } = useTranslation()
  const profile = useProfile()

  // Where the last plain tick landed, so a shift-click has something to reach
  // back to. A ref rather than state: it changes what the *next* click means
  // and nothing on screen depends on it.
  const anchor = useRef<string | null>(null)
  // Whether the click that is ticking a box held Shift. The box reports a
  // change and not the click behind it, so the cell around it notes the
  // modifier first; see the tick cell below.
  const extending = useRef(false)

  // The widths a person dragged, per machine. The header row is measured the
  // moment the first drag begins, so the columns nobody touched keep the
  // width they had when the layout goes fixed - see the hook for why it must.
  const widths = useColumnWidths<ColumnId>({
    initial: loadWidths,
    onChange: saveWidths,
    fallback: (id) => COLUMN_SPECS[id].naturalWidth,
    minWidth: MIN_COLUMN_WIDTH,
  })
  const header = useRef<HTMLTableRowElement>(null)

  const blocks = groupRows(rows, groupBy)
  const statusShown = columns.includes('status')

  // Under fixed layout the title is the one column left without a width, so
  // it takes whatever the window has spare - unless it was sized by hand,
  // in which case the table is exactly as wide as its columns. The minimum
  // width keeps the title from being squeezed to nothing when the others
  // add up to more than the window: the table scrolls instead.
  const titleSized = widths.isHandSized('title')

  /*
   * Whether the title is held against the left edge.
   *
   * Only while it is the first column. It is the one column that cannot be
   * turned off, but it CAN be dragged along the row, and a held cell has to
   * name the distance it stops at - `left: 36px`, the tick's width. Dragged
   * into third place it would stop 36px from the edge with two columns
   * sliding underneath it, which is worse than not holding it at all. So the
   * hold follows the usual arrangement and lets go of an unusual one.
   */
  const titleStuck = columns[0] === 'title'
  const total =
    SELECT_WIDTH + MENU_WIDTH + columns.reduce((sum, id) => sum + (widths.widthOf(id) ?? 0), 0)
  const colWidth = (id: ColumnId): number | undefined => {
    if (!widths.sized) return undefined
    if (id === 'title' && !titleSized) return undefined
    return widths.widthOf(id)
  }

  // The label a block carries. A status or tier the profile has since dropped
  // still names its block by its bare key rather than vanishing — the works are
  // real even when the vocabulary moved on.
  const groupLabel = (key: string | null) => {
    if (key === null) return t('catalogue.groupNone')
    return groupBy === 'status'
      ? labelOf(allOf(profile.config, 'statuses'), key)
      : labelOf(allOf(profile.config, 'tiers'), key)
  }

  // Only what is on screen can be ticked by the header box: filtering something
  // out and then selecting "all" must not reach it.
  const shownIds = rows.map((row) => row.work_id)
  const chosen = shownIds.filter((id) => selected.has(id))
  const allChosen = chosen.length > 0 && chosen.length === shownIds.length

  const toggleRow = (workId: string, extend: boolean) => {
    const next = new Set(selected)

    // Shift-click takes everything between, in the order the table is showing —
    // which is the order a person sees and means, not the order the ids happen
    // to be in.
    const from = anchor.current
    if (extend && from !== null && from !== workId) {
      const start = shownIds.indexOf(from)
      const end = shownIds.indexOf(workId)
      if (start !== -1 && end !== -1) {
        const [lo, hi] = start < end ? [start, end] : [end, start]
        // The anchor's own state decides the run: dragging out of a selection
        // clears the span, dragging out of an empty one fills it.
        const adding = selected.has(from)
        for (const id of shownIds.slice(lo, hi + 1)) {
          if (adding) next.add(id)
          else next.delete(id)
        }
        onSelectionChange(next)
        return
      }
    }

    if (!next.delete(workId)) next.add(workId)
    anchor.current = workId
    onSelectionChange(next)
  }

  const toggleAll = () => onSelectionChange(new Set(allChosen ? [] : shownIds))

  // One list, read by both ways into a row: the three dots and the right
  // click. Written once so the two can never come to disagree about what a
  // row can do.
  const actionsFor = (row: CatalogueRow): RowAction[] => [
    {
      key: 'score',
      label: t('catalogue.action.score'),
      onSelect: () => onSelect(row.work_id, 'score'),
    },
    {
      key: 'schedule',
      label: t('catalogue.action.schedule'),
      onSelect: () => onSelect(row.work_id, 'releases'),
    },
    {
      key: 'delete',
      label: t('catalogue.action.delete'),
      danger: true,
      onSelect: () => onDelete([row.work_id]),
    },
  ]

  const span = columns.length + 2

  return (
    <Panel className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
      <ScrollArea label={t('nav.catalogue')} className="flex-1">
        {/* `min-width`, never `width`, once columns have been sized by hand.
            A table given an explicit width WIDER than the box it scrolls in
            takes that width as its own edge — and `sticky right-0` holds a
            cell against the edge of the TABLE, which in that case sits off
            screen. The row menu then travelled with the rows, which is the
            one thing holding it was for. `min-w-max` says the same thing
            without ever declaring an edge past the viewport: the table is as
            wide as its columns need and no wider, and the held cell stops at
            the right edge of the scrolling box, where the eye is. */}
        <Table
          className={cn('min-w-max', widths.sized && 'table-fixed')}
          style={widths.sized ? { minWidth: total } : undefined}
        >
          {/* The widths live on the columns, not the cells: one `<col>` per
              drawn column, and only while something was sized by hand - until
              then the browser lays the table out from its contents as it
              always did, and nothing here is in its way. */}
          <colgroup>
            <col style={widths.sized ? { width: SELECT_WIDTH } : undefined} />
            {columns.map((id) => (
              <col
                key={id}
                style={colWidth(id) === undefined ? undefined : { width: colWidth(id) }}
              />
            ))}
            <col style={widths.sized ? { width: MENU_WIDTH } : undefined} />
          </colgroup>
          {/* The headings stay while the rows move under them: a table long
              enough to need scrolling is one whose columns must remain named.
              On the panel's ground, which is what the rows slide under, and
              above the held cells of the body - a sticky heading is a layer
              of its own, and the rows' held tick and title (`z-1`) otherwise
              slid over its corner. dowel's `z-sticky` names a variable
              Tailwind does not make a class of, so the height is said here. */}
          <TableHead sticky className="z-3 bg-raise">
            <tr ref={header} className="border-b border-line caption">
              {/* The tick column carries the same side padding as every other
                  cell: with none, the box sat flush against the star in the
                  next one and the two read as one control. */}
              <TableHeader className={cn('w-9 pr-2 pl-3', STUCK_HEADING, 'left-0')}>
                <Checkbox
                  checked={allChosen}
                  // Some but not all: the box shows neither state, because it is
                  // neither, and clicking it takes the rest.
                  indeterminate={chosen.length > 0 && !allChosen}
                  onCheckedChange={toggleAll}
                  aria-label={t('catalogue.selectAll')}
                />
              </TableHeader>
              {columns.map((id) => {
                const spec = COLUMN_SPECS[id]
                return (
                  <ColumnHeader
                    key={id}
                    id={id}
                    sortable={spec.sort}
                    sort={sort}
                    onReorder={onReorder}
                    label={t(spec.label)}
                    align={spec.align}
                    width={spec.width}
                    funnel={
                      <ColumnFunnel
                        column={id}
                        filters={columnFilters}
                        onChange={onColumnFilters}
                      />
                    }
                    widths={widths}
                    onMeasure={() => {
                      if (header.current) widths.measure(measureColumns<ColumnId>(header.current))
                    }}
                    stuck={id === 'title' && titleStuck}
                  />
                )
              })}
              <TableHeader className={cn('w-10', STUCK_HEADING, 'right-0')} />
            </tr>
          </TableHead>
          {/* Nothing matched, and the headings stay above the gap.

              This used to return the empty state INSTEAD of the table, so
              narrowing a filter to nothing took the whole apparatus away with
              the rows - the funnel that was just used, and every other heading,
              vanished with them. What is left then looks less like an answer
              than like a screen that broke: the filter cannot be widened from
              the controls that set it, because they are gone. The table is the
              furniture; only its contents are missing. */}
          {rows.length === 0 && (
            <TableBody>
              <tr>
                <td colSpan={span} className="p-0">
                  <EmptyState
                    variant="filtered"
                    title={t('empty.worksFiltered')}
                    body={t('empty.worksFilteredBody')}
                    action={<Button onClick={onClearFilters}>{t('empty.clearFilters')}</Button>}
                  />
                </td>
              </tr>
            </TableBody>
          )}

          {rows.length > 0 &&
            blocks.map((block) => {
              const key = block.key ?? GROUPLESS
              const folded = groupBy !== 'none' && collapsed.has(key)

              return (
                <TableBody key={key}>
                  {groupBy !== 'none' && (
                    <tr className="border-b border-line bg-softer">
                      {/* The whole band folds its block, not only its words: the
                          heading of a block is a row, and a row is pressed
                          anywhere along it. The count stays beside the words
                          rather than in the row's end: the band is as wide as
                          the table, and a table scrolled sideways would part
                          the two by a screen. */}
                      <td colSpan={span} className="px-1">
                        <RowButton
                          onClick={() => onToggleGroup(key)}
                          aria-expanded={!folded}
                          className="rounded-sm py-0.5"
                          start={
                            <ChevronRight
                              className={cn(
                                'size-3.5 text-faint transition-transform',
                                !folded && 'rotate-90',
                              )}
                              aria-hidden
                            />
                          }
                        >
                          {/* The mockup's band: a caption and its count, on
                              the softer ground of a heading within the table. */}
                          <span className="caption tabular-nums">
                            {`${groupLabel(block.key)} · ${block.rows.length}`}
                          </span>
                        </RowButton>
                      </td>
                    </tr>
                  )}

                  {!folded &&
                    block.rows.map((row) => (
                      <WorkRow
                        key={row.work_id}
                        actions={actionsFor(row)}
                        selected={selected.has(row.work_id)}
                        onOpen={() => onSelect(row.work_id)}
                      >
                        <TableCell
                          className={cn('pr-2 pl-3', STUCK_LEFT_TICK)}
                          // The change carries no modifier, so the click does: the
                          // cell reads Shift on the way down, before the box hears
                          // the click and changes.
                          onClickCapture={(event) => {
                            extending.current = event.shiftKey
                          }}
                          onClick={(event) => event.stopPropagation()}
                        >
                          <Checkbox
                            checked={selected.has(row.work_id)}
                            onCheckedChange={() => {
                              toggleRow(row.work_id, extending.current)
                              // Spent: a tick by keyboard has no click to set it,
                              // and must not inherit the last one's Shift.
                              extending.current = false
                            }}
                            aria-label={t('catalogue.select', { title: row.title })}
                          />
                        </TableCell>

                        {columns.map((id) => (
                          <CatalogueCell
                            key={id}
                            column={id}
                            row={row}
                            kindNarrowed={kindNarrowed}
                            statusShown={statusShown}
                            titleStuck={titleStuck}
                          />
                        ))}

                        <TableCell className={cn('px-1 text-right', STUCK_RIGHT_MENU)}>
                          <RowMenu
                            label={t('catalogue.rowMenu', { title: row.title })}
                            actions={actionsFor(row)}
                          />
                        </TableCell>
                      </WorkRow>
                    ))}
                </TableBody>
              )
            })}
        </Table>
      </ScrollArea>
    </Panel>
  )
}

/** The key standing in for "no value", which a `Map` cannot hold as `null`. */
const GROUPLESS = ' none'
