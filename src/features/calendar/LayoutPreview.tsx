import { useTranslation } from 'react-i18next'
import type { Placement } from '@/lib/api/types'
import { formatDay } from '@/lib/format'
import { Button } from '@/components/ui/button'

interface Props {
  placements: Placement[]
  onCancel: () => void
  onBook: () => void
  booking: boolean
}

/**
 * The auto-layout's plan, waiting for an answer: what would land where, said
 * in one line and drawn as ghosts in the month.
 *
 * Under the screen's head rather than in it: the head is what the month
 * shows and what can be done to it, and this is a question that outranks all
 * of that while it is up. Booking applies exactly the previewed array - the
 * backend refuses it whole if the calendar moved in between.
 */
export function LayoutPreview({ placements, onCancel, onBook, booking }: Props) {
  const { t } = useTranslation()
  // Said the way the rest of the window says a day, not as the ISO the plan
  // carries. The plan is in date order, so its ends are its first and last.
  const first = placements[0]?.date
  const last = placements[placements.length - 1]?.date

  return (
    <div className="flex w-full flex-wrap items-center gap-3 rounded-md border border-accent bg-accent-soft px-3 py-2 text-sm">
      <span className="flex-1">
        {t('calendar.layoutPreview', {
          count: placements.length,
          from: first === undefined ? '' : formatDay(first),
          to: last === undefined ? '' : formatDay(last),
        })}
      </span>
      <Button size="sm" onClick={onCancel}>
        {t('dialog.cancel')}
      </Button>
      <Button size="sm" variant="primary" disabled={booking} onClick={onBook}>
        {t('calendar.layoutApply')}
      </Button>
    </div>
  )
}
