import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type HTMLAttributes,
  type PointerEvent as ReactPointerEvent,
} from 'react'
import { cn } from 'dowel-ui'

/*
 * Drag a row of a vertical list up or down to put it somewhere else.
 *
 * The columns in a column picker, the stops of a dial, the roles of a
 * profile. A hook and a grip rather than a list component: the rows are
 * already something else's - a menu's items, a form's fields - and a
 * component wrapping them would have to reproduce whatever that something
 * else does.
 *
 * Pointer events, not HTML5 drag-and-drop, for the reason given at
 * ColumnResizeHandle: a desktop shell that takes file drops never lets a
 * `dragstart` reach the page. The grip takes pointer capture on pointerdown,
 * so the rows underneath never see the drag and a menu's own highlighting
 * does not flicker down the list as the pointer crosses it.
 *
 * Nothing moves until the pointer is let go. Live reordering looks better
 * for a second and costs a write per crossed row - to a profile, that is a
 * request per row - and the row being dragged has already been picked up,
 * so the reader is watching the line that says where it lands. The hook
 * reports that line as `slot`, and the caller draws it.
 *
 * The keyboard's way is Alt with an arrow on the focused row. Plain arrows
 * are how a list is walked, and taking them for moving would leave no way
 * to walk it; the modifier is the one screen readers and editors already
 * use for "move this line".
 */

export interface ReorderOptions<K extends string> {
  /** The ids in their current order. Every row carries its id in
   * `data-reorder-id`, which `rowProps` sets. */
  order: readonly K[]
  /** Put `id` at index `to` of the resulting list. Called once per drop or
   * per keypress, never during a drag. */
  onMove: (id: K, to: number) => void
  disabled?: boolean
}

export interface Reorder<K extends string> {
  /** Spread on the element holding the rows: the rows are found under it.
   * A callback ref rather than a ref object, so that nothing ref-shaped is
   * handed back and the compiler does not take the whole result for one. */
  listProps: { ref: (element: HTMLElement | null) => void }
  /** The row being dragged, or null. */
  dragging: K | null
  /** Where a drop would go, as an insertion point: between the rows at
   * `slot - 1` and `slot`, in the order as it is. Null while nothing is
   * dragged, or while the drop would change nothing. */
  slot: number | null
  /** The insertion point's distance from the top of the list element, in
   * pixels, for drawing the line. */
  slotOffset: number | null
  /** Spread on the grip: the pointer path. */
  gripProps: (id: K) => (HTMLAttributes<HTMLElement>)
  /** Spread on the row: the id and the keyboard path. The `ref` is a
   * callback that returns its cleanup, as React 19 allows; a host that merges
   * refs and drops the cleanup leaves a listener on a node that is gone,
   * which is harmless. */
  rowProps: (id: K) => {
    'data-reorder-id': K
    ref: (row: HTMLElement | null) => void | (() => void)
  }
}

export function useReorder<K extends string>({ order, onMove, disabled }: ReorderOptions<K>): Reorder<K> {
  const listRef = useRef<HTMLElement | null>(null)
  const [dragging, setDragging] = useState<K | null>(null)
  // The slot and where to draw it, settled together in the move handler: the
  // boxes it is read from are refs, and refs are not for reading in render.
  const [drop, setDrop] = useState<{ slot: number; offset: number } | null>(null)
  // The rows' boxes as they were when the drag began; nothing moves during
  // it, so reading them once is reading them right.
  const boxes = useRef<{ top: number; bottom: number }[]>([])
  const listTop = useRef(0)

  // The slot under a pointer: how many rows have their middle above it.
  const slotAt = (y: number): number => {
    let count = 0
    for (const box of boxes.current) if ((box.top + box.bottom) / 2 < y) count += 1
    return count
  }

  // A slot is an insertion point in the list as drawn; the move wants the
  // index in the list as it will be, with the row gone from where it was.
  const destination = (from: number, at: number): number => (at > from ? at - 1 : at)

  const begin = (id: K, event: ReactPointerEvent<HTMLElement>) => {
    if (disabled || event.button !== 0) return
    const list = listRef.current
    if (list === null) return
    event.preventDefault()
    event.stopPropagation()

    listTop.current = list.getBoundingClientRect().top
    // By attribute rather than by selector, so an id needs no escaping.
    const rows = [...list.querySelectorAll<HTMLElement>('[data-reorder-id]')]
    boxes.current = order.map((rowId) => {
      const box = rows.find((row) => row.getAttribute('data-reorder-id') === rowId)?.getBoundingClientRect()
      return box ? { top: box.top, bottom: box.bottom } : { top: 0, bottom: 0 }
    })
    event.currentTarget.setPointerCapture(event.pointerId)
    setDragging(id)
    setDrop(null)
  }

  // The insertion point's distance from the top of the list: the top of the
  // row it goes before, or the bottom of the last row.
  const offsetOf = (at: number): number => {
    const rows = boxes.current
    const edge = at < rows.length ? rows[at]?.top : rows[rows.length - 1]?.bottom
    return (edge ?? 0) - listTop.current
  }

  const move = (id: K, event: ReactPointerEvent<HTMLElement>) => {
    if (dragging !== id) return
    const from = order.indexOf(id)
    const at = slotAt(event.clientY)
    // Dropping a row back where it is - the slot just above or just below
    // itself - is no move, and drawing a line there would promise one.
    setDrop(destination(from, at) === from ? null : { slot: at, offset: offsetOf(at) })
  }

  const end = (id: K, event: ReactPointerEvent<HTMLElement>) => {
    if (dragging !== id) return
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId)
    }
    const from = order.indexOf(id)
    const at = slotAt(event.clientY)
    const to = destination(from, at)
    setDragging(null)
    setDrop(null)
    if (event.type !== 'pointercancel' && to !== from) onMove(id, to)
  }

  // Rebuilt every render on purpose: they close over the drag state and the
  // order, and memoising them would mean listing both and getting one wrong.
  const gripProps = (id: K): HTMLAttributes<HTMLElement> => ({
    onPointerDown: (event) => begin(id, event),
    onPointerMove: (event) => move(id, event),
    onPointerUp: (event) => end(id, event),
    onPointerCancel: (event) => end(id, event),
    // The click the drop leaves behind must not reach the row: in a menu it
    // would toggle the item that was only meant to be moved.
    onClick: (event) => event.stopPropagation(),
  })

  // The keyboard path is a native listener on the row, not a React one. A
  // menu popup handles the arrow keys itself and stops them on the way up,
  // so a React `onKeyDown` on the item - dispatched from the root, above the
  // popup - never hears them; a listener on the row itself fires at the
  // target first, whatever the ancestors do afterwards. The listener reads
  // the row's id and the latest order off refs rather than closing over
  // them, so one stable callback serves every row and nothing is re-bound
  // on each render.
  const latest = useRef({ order, onMove, disabled })
  useEffect(() => {
    latest.current = { order, onMove, disabled }
  })

  const listen = useCallback((row: HTMLElement | null) => {
    if (row === null) return
    const onKeyDown = (event: KeyboardEvent) => {
      const { order: current, onMove: put, disabled: off } = latest.current
      if (off || !event.altKey) return
      const step = event.key === 'ArrowUp' ? -1 : event.key === 'ArrowDown' ? 1 : 0
      if (step === 0) return
      const id = row.getAttribute('data-reorder-id') as K | null
      if (id === null) return
      const to = current.indexOf(id) + step
      if (to < 0 || to >= current.length) return
      // Stopped here so the list's own arrow handling does not also walk the
      // focus off the row that was just moved.
      event.preventDefault()
      event.stopPropagation()
      put(id, to)
    }
    row.addEventListener('keydown', onKeyDown)
    return () => row.removeEventListener('keydown', onKeyDown)
  }, [])

  const rowProps = (id: K) => ({
    'data-reorder-id': id,
    ref: listen,
  })

  return {
    listProps: {
      ref: (element) => {
        listRef.current = element
      },
    },
    dragging,
    slot: drop?.slot ?? null,
    slotOffset: drop?.offset ?? null,
    gripProps,
    rowProps,
  }
}

/** The handle a row is picked up by. Decorative to a screen reader - the
 * keyboard path is on the row itself - so it carries no role and no label. */
export function ReorderGrip({ className, ...props }: HTMLAttributes<HTMLElement>) {
  return (
    <span
      aria-hidden
      data-reorder-grip
      className={cn(
        'inline-flex shrink-0 cursor-grab touch-none select-none text-faint',
        'active:cursor-grabbing [&_svg]:size-3.5',
        className,
      )}
      {...props}
    >
      <svg viewBox="0 0 16 16" fill="currentColor" aria-hidden>
        <circle cx="6" cy="3.5" r="1.2" />
        <circle cx="10" cy="3.5" r="1.2" />
        <circle cx="6" cy="8" r="1.2" />
        <circle cx="10" cy="8" r="1.2" />
        <circle cx="6" cy="12.5" r="1.2" />
        <circle cx="10" cy="12.5" r="1.2" />
      </svg>
    </span>
  )
}

/** The line where a dragged row would land. Positioned by the caller from
 * `slotOffset`, inside the element `listProps` is on - which has to be
 * positioned itself. */
export function ReorderIndicator({ offset, className }: { offset: number | null; className?: string }) {
  if (offset === null) return null
  return (
    <div
      aria-hidden
      data-reorder-indicator
      className={cn('pointer-events-none absolute inset-x-1 h-0.5 -translate-y-px rounded-full bg-accent', className)}
      style={{ top: offset }}
    />
  )
}
