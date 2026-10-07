import type { GraphRow } from '@/lib/versionTree'
import { cn } from '@/lib/utils'

/** How wide one column of the gutter is, in pixels. */
const STEP = 12

interface Props {
  row: GraphRow
  /** How many columns the whole gutter has, so every row is as wide. */
  columns: number
  /** The work's current version: its dot is drawn in the accent, the way
   *  its row says "current". */
  current: boolean
  /** Said on hover: which version this one was written from. */
  title?: string
}

/**
 * One row of the tree beside the list of versions (`lib/versionTree`): the
 * version's dot, the line down to the version it was written from, the lines
 * passing by, and the ones arriving from versions written from it.
 *
 * Drawn per row rather than as one picture over the list, so it moves with
 * the rows it belongs to and needs no measuring. The lines reach two pixels
 * past the row at each end, across the gap the list leaves between rows, and
 * keep their width however tall the row is drawn.
 */
export function GraphCell({ row, columns, current, title }: Props) {
  const width = columns * STEP
  const x = (column: number) => column * STEP + STEP / 2
  const line = (x1: number, y1: number, x2: number, y2: number) => ({
    x1,
    y1,
    x2,
    y2,
    vectorEffect: 'non-scaling-stroke' as const,
  })

  return (
    <span
      title={title}
      data-graph-column={row.column}
      className="relative shrink-0 self-stretch"
      style={{ width }}
    >
      <svg
        aria-hidden
        className="absolute inset-x-0 -top-0.5 h-[calc(100%+4px)] w-full text-line-2"
        viewBox={`0 0 ${width} 100`}
        preserveAspectRatio="none"
        fill="none"
        stroke="currentColor"
        strokeWidth={1.5}
      >
        {row.through.map((column) => (
          <line key={`through-${column}`} {...line(x(column), 0, x(column), 100)} />
        ))}
        {row.up && <line {...line(x(row.column), 0, x(row.column), 50)} />}
        {row.down && <line {...line(x(row.column), 50, x(row.column), 100)} />}
        {row.joins.map((column) => (
          <path
            key={`join-${column}`}
            d={`M ${x(column)} 0 C ${x(column)} 35 ${x(row.column)} 15 ${x(row.column)} 50`}
            vectorEffect="non-scaling-stroke"
          />
        ))}
      </svg>
      <span
        aria-hidden
        className={cn(
          'absolute top-1/2 size-2 -translate-x-1/2 -translate-y-1/2 rounded-full',
          current ? 'bg-accent' : 'bg-faint',
        )}
        style={{ left: x(row.column) }}
      />
    </span>
  )
}
