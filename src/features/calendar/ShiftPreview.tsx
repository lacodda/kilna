import { useTranslation } from 'react-i18next'
import type { ShiftPlan } from '@/lib/api/types'
import { formatDay } from '@/lib/format'
import { Button } from '@/components/ui/button'

interface Props {
  plan: ShiftPlan
  from: string
  /** Positive is later, negative earlier. */
  days: number
  onCancel: () => void
  onShift: () => void
  shifting: boolean
}

/**
 * A shift waiting for an answer: how many releases would move, by how much,
 * what stays, and what would land on a closed day - in one line, with the
 * moved releases drawn as ghosts on their new days.
 *
 * The layout's bar, for a plan already booked. Shifting books exactly the
 * moves it shows; the backend refuses them whole if the calendar moved since.
 */
export function ShiftPreview({ plan, from, days, onCancel, onShift, shifting }: Props) {
  const { t } = useTranslation()
  const moved = plan.moves.length
  const said = [
    t(days > 0 ? 'shift.previewLater' : 'shift.previewEarlier', {
      count: moved,
      days: t('shift.daysCount', { count: Math.abs(days) }),
      from: formatDay(from),
    }),
    plan.kept.length > 0 ? t('shift.previewKept', { count: plan.kept.length }) : null,
    plan.closed.length > 0 ? t('shift.previewClosed', { count: plan.closed.length }) : null,
  ].filter((part) => part !== null)

  return (
    <div className="flex w-full flex-wrap items-center gap-3 rounded-md border border-accent bg-accent-soft px-3 py-2 text-sm">
      <span className="flex-1">{said.join(' ')}</span>
      <Button size="sm" onClick={onCancel}>
        {t('dialog.cancel')}
      </Button>
      <Button size="sm" variant="primary" disabled={shifting || moved === 0} onClick={onShift}>
        {t('shift.apply')}
      </Button>
    </div>
  )
}
