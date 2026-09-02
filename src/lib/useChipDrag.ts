import { useCallback, useEffect, useRef, useState } from 'react'
import { EDGE_DELAY, edgeOf, ghostAt, isDrag, type EdgeSide } from '@/lib/drag'

/** What is being dragged, and where it is right now. */
export interface Dragging {
  /** The release under the pointer. */
  id: string
  /** Where to draw the chip that follows the pointer. */
  ghost: { left: number; top: number }
  /** The size the ghost should be — the chip's own, so it does not resize. */
  size: { width: number; height: number }
}

interface Options {
  /** The grid, for working out when the pointer is near an edge. */
  gridRef: React.RefObject<HTMLElement | null>
  /** Turn the month while the pointer rests in an edge strip. */
  onEdge: (step: -1 | 1) => void
  /** The release was let go over this element, or over nothing. */
  onDrop: (id: string, target: Element | null) => void
}

/**
 * Picking a chip up and carrying it.
 *
 * Pointer events rather than HTML5 drag and drop, for three reasons found the
 * hard way. The native handler is switched off in `tauri.conf.json` because it
 * ate the whole gesture on Windows — that is one dependency this does not want
 * back. The thing that follows the pointer is a real chip, drawn by React,
 * rather than a bitmap the browser snapshots and nobody can change afterwards.
 * And the month has to turn when the pointer nears the edge of the grid, which
 * is not scrolling and no drag library offers it.
 *
 * A press is only a drag once it has travelled `DRAG_THRESHOLD`. Until then it
 * is still a click, because the chip is also the button that opens the release.
 */
export function useChipDrag({ gridRef, onEdge, onDrop }: Options) {
  const [dragging, setDragging] = useState<Dragging | null>(null)

  // Everything the move and up handlers need, kept out of state so that
  // changing it does not re-render on every pointer move.
  const press = useRef<{
    id: string
    from: { x: number; y: number }
    grab: { x: number; y: number }
    size: { width: number; height: number }
    started: boolean
  } | null>(null)

  const edge = useRef<{ side: EdgeSide; timer: number | null }>({ side: 0, timer: null })

  const stopEdge = useCallback(() => {
    if (edge.current.timer !== null) window.clearInterval(edge.current.timer)
    edge.current = { side: 0, timer: null }
  }, [])

  /** Start, stop or leave the month turning, following the pointer's side. */
  const followEdge = useCallback(
    (x: number) => {
      const bounds = gridRef.current?.getBoundingClientRect()
      const side = bounds === undefined ? 0 : edgeOf(x, bounds)
      if (side === edge.current.side) return

      stopEdge()
      if (side === 0) return

      // The first turn waits as long as the ones after it: crossing the strip
      // on the way to a day near the edge must not set anything off.
      edge.current = {
        side,
        timer: window.setInterval(() => onEdge(side), EDGE_DELAY),
      }
    },
    [gridRef, onEdge, stopEdge],
  )

  const begin = useCallback(
    (event: React.PointerEvent, id: string) => {
      // The left button only, and never a second press while one is in the air.
      if (event.button !== 0 || press.current !== null) return

      const box = event.currentTarget.getBoundingClientRect()
      press.current = {
        id,
        from: { x: event.clientX, y: event.clientY },
        grab: { x: event.clientX - box.left, y: event.clientY - box.top },
        size: { width: box.width, height: box.height },
        started: false,
      }
    },
    [],
  )

  /** Let go of everything, whether or not a drag ever started. */
  const cancel = useCallback(() => {
    press.current = null
    stopEdge()
    setDragging(null)
  }, [stopEdge])

  useEffect(() => {
    const move = (event: PointerEvent) => {
      const held = press.current
      if (held === null) return

      const at = { x: event.clientX, y: event.clientY }
      if (!held.started) {
        if (!isDrag(held.from, at)) return
        held.started = true
      }

      // Once carrying, the page must not also select text or scroll under it.
      event.preventDefault()
      setDragging({ id: held.id, ghost: ghostAt(at, held.grab), size: held.size })
      followEdge(at.x)
    }

    const up = (event: PointerEvent) => {
      const held = press.current
      if (held === null) return

      const started = held.started
      // Read before the ghost goes: it sits under the pointer and would be
      // the answer to `elementFromPoint` instead of the day underneath.
      const target = started ? document.elementFromPoint(event.clientX, event.clientY) : null
      cancel()
      if (started) onDrop(held.id, target)
    }

    // Escape puts it back rather than dropping it somewhere by accident — the
    // same way out every other overlay in the app offers.
    const key = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && press.current !== null) cancel()
    }

    window.addEventListener('pointermove', move, { passive: false })
    window.addEventListener('pointerup', up)
    window.addEventListener('pointercancel', cancel)
    window.addEventListener('keydown', key)
    return () => {
      window.removeEventListener('pointermove', move)
      window.removeEventListener('pointerup', up)
      window.removeEventListener('pointercancel', cancel)
      window.removeEventListener('keydown', key)
      stopEdge()
    }
  }, [cancel, followEdge, onDrop, stopEdge])

  return { dragging, begin, cancel }
}
