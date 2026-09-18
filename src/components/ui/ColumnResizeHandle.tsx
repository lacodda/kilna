import { useCallback, useRef, useState, type PointerEvent as ReactPointerEvent } from 'react'
import { cn } from 'dowel-ui'

/*
 * ColumnResizeHandle.
 *
 * The strip on the right edge of a header cell that a column is dragged
 * wider or narrower by, and the hook that keeps the widths it produces.
 *
 * Pointer events rather than HTML5 drag-and-drop. A desktop shell that takes
 * file drops for itself never lets a `dragstart` reach the page, so the
 * native API is a handle that does nothing there; pointer capture on the
 * handle also keeps the drag alive when the pointer runs ahead of the cell,
 * which at any speed above a crawl it does.
 *
 * The starting width is measured from the cell rather than taken as a prop:
 * on pointerdown the handle reads its parent's box, and every move reports
 * that width plus the distance travelled. So the handle has no `width` to be
 * handed and cannot disagree with what is on screen. Double-click is the
 * reset, because a handle two pixels wide has no room for a second control.
 *
 * The widths themselves are the hook's, and there are two maps in it rather
 * than one. A table with any hand-set width has to switch to
 * `table-layout: fixed`, and under fixed layout every column needs a width
 * or the browser shares the free space equally between those without one -
 * which is how a date column ends up as wide as the title. So beside the
 * widths that were dragged, the hook keeps the widths every other column
 * measured the moment the first drag began, and `widthOf` answers from one
 * or the other. The caller draws one `<col>` per column from it and leaves
 * the one column meant to take the remaining space without a width.
 */

export interface ColumnResizeHandleProps {
  /** Names the column for assistive technology: "Resize the Tier column". */
  label: string
  /** Shown on hover: how to drag and how to reset. */
  hint?: string
  /** The width while dragging, and once more with `done` when the pointer is
   * let go - the moment to persist. */
  onResize: (width: number, done: boolean) => void
  /** Double-click: the column goes back to its natural width. */
  onReset: () => void
  /** Called on pointerdown, before the first `onResize` - the moment for the
   * caller to measure what every column is before the layout goes fixed. */
  onStart?: () => void
  /** Narrower than this and the column is a stripe with nothing in it. */
  minWidth?: number
  className?: string
}

export function ColumnResizeHandle({
  label,
  hint,
  onResize,
  onReset,
  onStart,
  minWidth = 56,
  className,
}: ColumnResizeHandleProps) {
  // Where the drag began, in both senses: the pointer's x and the cell's width.
  const origin = useRef<{ x: number; width: number } | null>(null)
  const [dragging, setDragging] = useState(false)

  const widthAt = (event: ReactPointerEvent<HTMLElement>): number | null => {
    const from = origin.current
    if (from === null) return null
    return Math.max(minWidth, Math.round(from.width + event.clientX - from.x))
  }

  const begin = (event: ReactPointerEvent<HTMLElement>) => {
    // The primary button only: a right-click on the edge is the context
    // menu's, and a middle one is nobody's.
    if (event.button !== 0) return
    const cell = event.currentTarget.parentElement
    if (cell === null) return
    event.preventDefault()
    event.stopPropagation()
    onStart?.()
    origin.current = { x: event.clientX, width: cell.getBoundingClientRect().width }
    event.currentTarget.setPointerCapture(event.pointerId)
    setDragging(true)
  }

  const move = (event: ReactPointerEvent<HTMLElement>) => {
    const width = widthAt(event)
    if (width !== null) onResize(width, false)
  }

  const end = (event: ReactPointerEvent<HTMLElement>) => {
    const width = widthAt(event)
    origin.current = null
    setDragging(false)
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId)
    }
    if (width !== null) onResize(width, true)
  }

  return (
    // A span, not a button: a button in a header cell would be one more tab
    // stop per column for something a keyboard cannot usefully drive. The
    // separator role says what it is; the label says which column.
    <span
      role="separator"
      aria-orientation="vertical"
      aria-label={label}
      title={hint}
      data-dragging={dragging ? '' : undefined}
      onPointerDown={begin}
      onPointerMove={move}
      onPointerUp={end}
      onPointerCancel={end}
      onDoubleClick={(event) => {
        event.stopPropagation()
        onReset()
      }}
      // The click after the drag must not reach the header: on a sortable
      // column it would flip the sort every time a width was set.
      onClick={(event) => event.stopPropagation()}
      className={cn(
        // Wider to hit than to see: the visible line is the inner pixel, the
        // grab zone is the whole strip. `touch-action: none` is what lets a
        // touch drag the column instead of scrolling the table.
        'absolute inset-y-0 right-0 z-10 w-2 cursor-col-resize touch-none select-none',
        'after:absolute after:inset-y-1.5 after:right-0.5 after:w-px after:bg-line after:transition-colors',
        'hover:after:bg-accent data-[dragging]:after:bg-accent',
        className,
      )}
    />
  )
}

/** The widths of the columns the hook is asked about, by column id. */
export type Widths<K extends string> = Partial<Record<K, number>>

export interface ColumnWidthsOptions<K extends string> {
  /** What was dragged before - from storage, or nothing. A function is
   * called once, like `useState`'s. */
  initial: Widths<K> | (() => Widths<K>)
  /** Told the hand-set widths whenever one settles: persist them here. */
  onChange?: (widths: Widths<K>) => void
  /** The width of a column that was not on screen when the others were
   * measured - one turned on after the first drag. */
  fallback?: (id: K) => number
  minWidth?: number
}

export interface ColumnWidths<K extends string> {
  /** True while any width is hand-set: the table is in fixed layout. */
  sized: boolean
  /** The widths that were dragged. */
  hand: Widths<K>
  /** Whether this column's width is the person's rather than measured. */
  isHandSized: (id: K) => boolean
  /** A width for the `<col>`: dragged, else measured, else the fallback. */
  widthOf: (id: K) => number | undefined
  /** A width from the handle; `done` on the last one persists. */
  resize: (id: K, width: number, done?: boolean) => void
  /** Back to the natural width; with nothing else hand-set, back to auto layout. */
  reset: (id: K) => void
  /** The natural widths, taken once before the layout goes fixed. Ignored
   * once it has, because what is measured then is the fixed width. */
  measure: (cells: Iterable<[K, number]>) => void
}

export function useColumnWidths<K extends string>({
  initial,
  onChange,
  fallback,
  minWidth = 56,
}: ColumnWidthsOptions<K>): ColumnWidths<K> {
  const [hand, setHand] = useState<Widths<K>>(initial)
  const [natural, setNatural] = useState<Widths<K>>({})
  // A mirror the callbacks read, so a resize that settles in the same tick as
  // its last move persists the value that was just set and not the one from
  // the render before.
  const held = useRef(hand)
  const sized = Object.keys(hand).length > 0

  const commit = useCallback(
    (next: Widths<K>, persist: boolean) => {
      held.current = next
      setHand(next)
      if (persist) onChange?.(next)
    },
    [onChange],
  )

  const resize = useCallback(
    (id: K, width: number, done = false) => {
      commit({ ...held.current, [id]: Math.max(minWidth, Math.round(width)) }, done)
    },
    [commit, minWidth],
  )

  const reset = useCallback(
    (id: K) => {
      const rest: Widths<K> = { ...held.current }
      delete rest[id]
      commit(rest, true)
      // With nothing hand-set the layout goes back to auto, and the next drag
      // measures afresh - the natural widths may have changed with the data.
      if (Object.keys(rest).length === 0) setNatural({})
    },
    [commit],
  )

  const measure = useCallback(
    (cells: Iterable<[K, number]>) => {
      if (Object.keys(held.current).length > 0) return
      const measured: Widths<K> = {}
      for (const [id, width] of cells) measured[id] = Math.round(width)
      setNatural(measured)
    },
    [],
  )

  return {
    sized,
    hand,
    isHandSized: (id) => hand[id] !== undefined,
    widthOf: (id) => hand[id] ?? natural[id] ?? fallback?.(id),
    resize,
    reset,
    measure,
  }
}

/**
 * The widths of a header row's cells, read off the screen.
 *
 * Each cell names its column in `data-column`; a cell without one - a
 * checkbox column, a row menu - is not a column the hook is asked about and
 * is skipped. Border-box widths, because that is what a `<col>` sets.
 */
export function measureColumns<K extends string>(
  row: HTMLElement,
  attribute = 'data-column',
): [K, number][] {
  const measured: [K, number][] = []
  for (const cell of row.querySelectorAll<HTMLElement>(`[${attribute}]`)) {
    const id = cell.getAttribute(attribute)
    if (id !== null) measured.push([id as K, cell.getBoundingClientRect().width])
  }
  return measured
}
