import { createPortal } from 'react-dom'
import type { ScheduledRelease } from '@/lib/api/types'
import type { Dragging } from '@/lib/useChipDrag'
import { SlotChip } from '@/features/calendar/SlotChip'

interface Props {
  dragging: Dragging
  slot: ScheduledRelease
  now: string
}

/**
 * The chip that follows the pointer.
 *
 * A real one, drawn by React rather than a bitmap the browser snapshots: it
 * keeps the work's colour, its marks and its title, and it can be styled while
 * it travels. Fixed to the viewport, so no ancestor's overflow can clip it -
 * the days clip their chips now, and the queue it may be carried to is beside
 * the month rather than inside it.
 */
export function CarriedChip({ dragging, slot, now }: Props) {
  return createPortal(
    <div
      // Above the overlays it may pass under; the scale is dowel's, not a
      // number picked here. Lifted onto the panel's ground, since the chip's
      // soft fill alone would let every day it crosses show through.
      className="pointer-events-none fixed rounded-sm bg-raise shadow-float [z-index:var(--z-overlay)]"
      style={{
        left: dragging.ghost.left,
        top: dragging.ghost.top,
        width: dragging.size.width,
      }}
    >
      <SlotChip
        slot={slot}
        date={slot.scheduled_at ?? now}
        now={now}
        dragging={false}
        onGrab={() => {}}
        onOpen={() => {}}
        asGhost
      />
    </div>,
    document.body,
  )
}
