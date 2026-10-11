import { useEffect, useRef, useState } from 'react'
import { useChipDrag } from '@/lib/useChipDrag'

/** What `over` holds while the pointer is over the queue rather than a day. */
export const QUEUE = 'queue'

/**
 * What a work row of the pane carries (v0.96): a work, not a release - it has
 * none yet, and gets one on the day it is let go over. Prefixed so one hook
 * carries both, and so a release id can never be read as a work's.
 */
const WORK = 'work:'

export function carryWork(workId: string): string {
  return `${WORK}${workId}`
}

/** The work a carried id names, or `null` for a release. */
export function carriedWork(id: string): string | null {
  return id.startsWith(WORK) ? id.slice(WORK.length) : null
}

/** The pointer handlers a place to land wears, so it can light up under a chip. */
export interface DropTarget {
  onPointerEnter: () => void
  onPointerLeave: () => void
}

interface Options {
  /** Turn the period on screen a step - a month, or a week - when the
   *  pointer rests at an edge of the grid. */
  onStep: (step: -1 | 1) => void
  /** Something let go over a day: a booked release, a queued one, or a work
   *  (`carriedWork`). */
  onMove: (id: string, date: string) => void
  /** Something let go over the queue. */
  onUnschedule: (id: string) => void
}

/**
 * A chip carried across the screen, and where it may land.
 *
 * Lifted out of the month into the screen because the month is no longer the
 * only place to land: the queue beside it takes a release back, the way the
 * mockup draws it, and the queue has to light up under the chip as the days
 * do. So the carried chip, and which place the pointer is over, belong to the
 * screen that holds both.
 *
 * Landing places are found by what the pointer is let go over rather than by
 * who registered: a day carries `data-day`, a place that returns a release to
 * the queue carries `data-queue-drop`.
 */
export function useCalendarDrag({ onStep, onMove, onUnschedule }: Options) {
  // The month, so the pointer reaching its edge can turn it.
  const gridRef = useRef<HTMLDivElement>(null)

  // The day under the pointer, the queue, or nothing. Set from where the
  // carried chip is rather than from each place's own hover, because the
  // ghost sits under the pointer and nothing can see through it.
  const [over, setOver] = useState<string | null>(null)
  // The same, for the edge timer, which fires between renders.
  const overRef = useRef(over)
  useEffect(() => {
    overRef.current = over
  })

  const { dragging, begin } = useChipDrag({
    gridRef,
    // Not while the chip is over the queue. A pointer past the month counts
    // as being in the edge strip it left through (`edgeOf`) - which, with the
    // queue standing to the right of the month, is the whole queue: a chip
    // held over it to be put back turned the month every half second.
    onEdge: (step) => {
      if (overRef.current !== QUEUE) onStep(step)
    },
    onDrop: (id, target) => {
      setOver(null)
      const day = target?.closest<HTMLElement>('[data-day]')?.dataset.day
      if (day !== undefined) {
        onMove(id, day)
        return
      }
      if (target?.closest('[data-queue-drop]') != null) onUnschedule(id)
    },
  })

  /** The handlers for the place called `key`: it is `over` while the pointer is. */
  const target = (key: string): DropTarget => ({
    onPointerEnter: () => setOver(key),
    onPointerLeave: () => setOver((current) => (current === key ? null : current)),
  })

  return { gridRef, dragging, begin, over, target }
}
