import { useTranslation } from 'react-i18next'
import { Check, CheckCheck } from 'lucide-react'
import type { ScheduledRelease } from '@/lib/api/types'
import { missing, needKey, sayNeed, urgency } from '@/lib/readiness'
import { useProfile } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { NeedGlyph } from '@/components/NeedGlyph'

interface Props {
  release: Pick<ScheduledRelease, 'readiness' | 'status' | 'work_kind' | 'kind'>
  /** Days until the slot, or null when the release holds no date. */
  daysLeft: number | null
  className?: string
}

// The colour belongs to the deadline, not to the gap: the same missing style
// prompt is a calm note a month out and a red flag two days before the slot.
const TONE: Record<ReturnType<typeof urgency>, string> = {
  calm: 'text-dim',
  soon: 'text-warn',
  urgent: 'text-bad',
}

/**
 * Readiness at a glance: two ticks it went out, one tick everything is there,
 * otherwise one glyph per gap - a page for a missing role, a gauge for a
 * missing score, a picture for a missing cover, a pen for a field nobody
 * wrote. The words live in the tooltip; the row has to survive at eleven
 * pixels on a calendar chip.
 */
export function ReadyMarks({ release, daysLeft, className }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()

  if (release.status === 'released') {
    return (
      <span className={cn('inline-flex shrink-0', className)} title={t('calendar.released')}>
        <CheckCheck aria-hidden className="size-3" />
      </span>
    )
  }

  if (release.readiness.ready) {
    return (
      <span className={cn('inline-flex shrink-0 text-good', className)} title={t('calendar.ready')}>
        <Check aria-hidden className="size-3" />
      </span>
    )
  }

  const gaps = missing(release.readiness)
  const said = t('calendar.notReadyHint', {
    list: gaps.map((gap) => sayNeed(gap, profile.config, release, t)).join(', '),
  })

  return (
    <span
      className={cn(
        'inline-flex shrink-0 items-center gap-0.5',
        TONE[urgency(daysLeft)],
        className,
      )}
      title={said}
    >
      {gaps.map((gap) => (
        <NeedGlyph key={needKey(gap)} need={gap} className="size-3" />
      ))}
      <span className="sr-only">{said}</span>
    </span>
  )
}
