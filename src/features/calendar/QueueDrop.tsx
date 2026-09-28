import { useTranslation } from 'react-i18next'
import { Undo2 } from 'lucide-react'
import type { DropTarget } from '@/features/calendar/useCalendarDrag'
import { cn } from '@/lib/utils'

interface Props {
  /** A booked release is in the air, and would land here. */
  active: boolean
  /** It is over this place right now. */
  over: boolean
  /** Its own handlers, where it is a landing place by itself rather than the
   * foot of one: inside the queue the whole pane lands, and a second set of
   * handlers on its foot would let go of the pane on the way out of the foot. */
  target?: DropTarget
  className?: string
}

/**
 * Where a booked release goes back to the queue: carried off the month and
 * let go here, its date is cleared and it waits again.
 *
 * Standing in the queue's foot, as the mockup draws it, rather than appearing
 * only under a dragged chip. A bin that stood there permanently invited the
 * question "what does this delete?"; this says what it does - the release
 * goes back where the queue is - and nothing is deleted. Accent, not red, for
 * the same reason.
 */
export function QueueDrop({ active, over, target, className }: Props) {
  const { t } = useTranslation()

  return (
    <div
      data-queue-drop
      onPointerEnter={target?.onPointerEnter}
      onPointerLeave={target?.onPointerLeave}
      className={cn(
        'flex h-control-sm w-full items-center justify-center gap-1.5 rounded-md border border-dashed px-3 text-xs transition-colors',
        over
          ? 'border-accent bg-accent-soft text-accent'
          : active
            ? 'border-accent text-dim'
            : 'border-line-2 text-faint',
        className,
      )}
    >
      <Undo2 aria-hidden className="size-3.5 shrink-0" />
      <span className="truncate">{t('calendar.dropToUnschedule')}</span>
    </div>
  )
}
