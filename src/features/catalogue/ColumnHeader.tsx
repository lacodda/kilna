import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { MIN_COLUMN_WIDTH, type ColumnId, type Sort, type SortColumn } from '@/lib/catalogue'
import { cn } from '@/lib/utils'
import { ColumnResizeHandle, type ColumnWidths } from '@/components/ui/column-resize-handle'
import { TableHeader, TableSortButton } from '@/components/ui/table'
import { ariaSort } from '@/components/ui/table-sort'
import { STUCK_HEADING, TITLE_HOLD } from './columns'

interface Props {
  id: ColumnId
  /** The sort it drives, or null for a column that answers no ordering question. */
  sortable: SortColumn | null
  sort: Sort
  onReorder: (column: SortColumn) => void
  label: string
  align?: 'left' | 'right'
  /** The class sizing it while the layout is still the browser's. */
  width?: string
  /** The column's funnel, or nothing. */
  funnel: ReactNode
  widths: ColumnWidths<ColumnId>
  /** Measure every header cell: called as a drag begins, before the first
   * width is set and the layout goes fixed. */
  onMeasure: () => void
  /** Whether this heading is held against the left edge with its column. */
  stuck?: boolean
}

/**
 * A column's heading: sorts when the column can, says which way it is
 * pointing, carries the funnel when the column has one, and ends in the handle
 * its width is dragged by.
 */
export function ColumnHeader({
  id,
  sortable,
  sort,
  onReorder,
  label,
  align = 'left',
  width,
  funnel,
  widths,
  onMeasure,
  stuck = false,
}: Props) {
  const { t } = useTranslation()

  return (
    // `aria-sort` goes on the cell rather than the button: it describes the
    // column, and a button is not a column. `data-column` is what the
    // measuring reads. `group` is for the funnel, which shows on hover of
    // the whole cell rather than of its own few pixels.
    <TableHeader
      data-column={id}
      aria-sort={sortable === null ? undefined : ariaSort(sort, sortable)}
      // Padding on both sides, not just the one the text runs towards: a
      // right-aligned column followed by a left-aligned one used to put its
      // last letter against the next header's first, and "Total"/"Scored" read
      // as one word. Overflow is clipped once widths are the person's: a
      // column dragged narrower than its heading shows the start of it.
      className={cn(
        'group relative overflow-hidden font-semibold whitespace-nowrap',
        width,
        // Held with its column, and above it: at the top-left corner the
        // heading and the first cell overlap, and the heading wins. A heading
        // that is not held is a layer of its own at the bottom: its resize
        // grip stands at `z-10` to be grabbed over the next heading, and
        // without a layer to keep it in, the grips of the headings sliding
        // under the held title drew over the title's name.
        stuck ? STUCK_HEADING : 'z-0',
      )}
      style={stuck ? TITLE_HOLD : undefined}
    >
      <span className={cn('flex items-center gap-1', align === 'right' && 'justify-end')}>
        {sortable === null ? (
          <span className="truncate">{label}</span>
        ) : (
          // dowel's TableSortHeader takes the whole cell, and this heading
          // holds two more controls - the funnel and the resize grip - which
          // inside its button would be buttons in a button. So the cell is
          // written here and the button stands beside them.
          <TableSortButton
            column={sortable}
            sort={sort}
            onSortChange={() => onReorder(sortable)}
            title={t('catalogue.sortBy', { column: label })}
            className="caption"
          >
            {label}
          </TableSortButton>
        )}
        {funnel}
      </span>
      <ColumnResizeHandle
        label={t('catalogue.resizeColumn', { column: label })}
        hint={t('catalogue.resizeHint')}
        minWidth={MIN_COLUMN_WIDTH}
        onStart={onMeasure}
        onResize={(px, done) => widths.resize(id, px, done)}
        onReset={() => widths.reset(id)}
      />
    </TableHeader>
  )
}
