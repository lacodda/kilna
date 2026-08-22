import { useTranslation } from 'react-i18next'
import { Check, CheckCheck, FileText, Gauge } from 'lucide-react'
import type { Readiness } from '@/lib/api'
import { missing, urgency } from '@/lib/readiness'
import { labelOf, useProfile } from '@/lib/useProfile'
import { cn } from '@/lib/utils'

interface Props {
  readiness: Readiness
  /** The release already went out; readiness is history. */
  released: boolean
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
 * otherwise one glyph per gap — a page for a missing role, a gauge for a
 * missing score. The words live in the tooltip; the row has to survive at
 * eleven pixels on a calendar chip.
 */
export function ReadyMarks({ readiness, released, daysLeft, className }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()

  if (released) {
    return (
      <span className={cn('inline-flex shrink-0', className)} title={t('calendar.released')}>
        <CheckCheck aria-hidden className="size-3" />
      </span>
    )
  }

  if (readiness.ready) {
    return (
      <span
        className={cn('inline-flex shrink-0 text-good', className)}
        title={t('calendar.ready')}
      >
        <Check aria-hidden className="size-3" />
      </span>
    )
  }

  const gaps = missing(readiness)
  const names = gaps.map((gap) =>
    gap === 'score' ? t('calendar.missingScore') : labelOf(profile.config.version_roles, gap),
  )

  return (
    <span
      className={cn('inline-flex shrink-0 items-center gap-0.5', TONE[urgency(daysLeft)], className)}
      title={t('calendar.notReadyHint', { list: names.join(', ') })}
    >
      {gaps.map((gap) =>
        gap === 'score' ? (
          <Gauge key={gap} aria-hidden className="size-3" />
        ) : (
          <FileText key={gap} aria-hidden className="size-3" />
        ),
      )}
      <span className="sr-only">{t('calendar.notReadyHint', { list: names.join(', ') })}</span>
    </span>
  )
}
