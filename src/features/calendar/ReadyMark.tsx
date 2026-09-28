import { useTranslation } from 'react-i18next'
import { Check, CheckCheck, FileText, Gauge } from 'lucide-react'
import type { Readiness } from '@/lib/api/types'
import { missing, urgency } from '@/lib/readiness'
import { allOf, labelOf, useProfile } from '@/lib/useProfile'
import { cn } from '@/lib/utils'

interface Props {
  readiness: Readiness
  /** The release already went out; readiness is history. */
  released: boolean
  /** Days until the slot. */
  daysLeft: number
}

// The colour belongs to the deadline, not to the gap - the same scale as
// `ReadyMarks`, so the queue and the month say urgency in one voice.
const TONE: Record<ReturnType<typeof urgency>, string> = {
  calm: 'text-dim',
  soon: 'text-warn',
  urgent: 'text-bad',
}

/**
 * Readiness in one glyph, for a chip that is one line.
 *
 * `ReadyMarks` draws a glyph per gap, which is right for the queue's roomy
 * rows and was three glyphs on a chip a hundred pixels wide. Here the first
 * gap stands for all of them - a page for a missing role, a gauge for a
 * missing score - coloured by how close the day is; the whole list is in the
 * tooltip, in the hover card and in the release dialog.
 */
export function ReadyMark({ readiness, released, daysLeft }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()

  if (released) {
    return (
      <span className="inline-flex shrink-0" title={t('calendar.released')}>
        <CheckCheck aria-hidden className="size-3" />
      </span>
    )
  }

  if (readiness.ready) {
    return (
      <span className="inline-flex shrink-0 text-good" title={t('calendar.ready')}>
        <Check aria-hidden className="size-3" />
      </span>
    )
  }

  const gaps = missing(readiness)
  const said = t('calendar.notReadyHint', {
    list: gaps
      .map((gap) =>
        gap === 'score'
          ? t('calendar.missingScore')
          : labelOf(allOf(profile.config, 'version_roles'), gap),
      )
      .join(', '),
  })
  const Glyph = gaps[0] === 'score' ? Gauge : FileText

  return (
    <span className={cn('inline-flex shrink-0', TONE[urgency(daysLeft)])} title={said}>
      <Glyph aria-hidden className="size-3" />
      <span className="sr-only">{said}</span>
    </span>
  )
}
