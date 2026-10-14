import { useTranslation } from 'react-i18next'
import { Check, CheckCheck } from 'lucide-react'
import type { ScheduledRelease } from '@/lib/api/types'
import { missing, sayNeed, urgency } from '@/lib/readiness'
import { useProfile } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { NeedGlyph } from '@/components/NeedGlyph'

interface Props {
  release: Pick<ScheduledRelease, 'readiness' | 'status' | 'work_kind' | 'kind'>
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
 * gap stands for all of them - a page, a gauge, a picture or a pen, the same
 * glyphs `ReadyMarks` draws - coloured by how close the day is; the whole
 * list is in the tooltip, in the hover card and in the release dialog.
 */
export function ReadyMark({ release, daysLeft }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()

  if (release.status === 'released') {
    return (
      <span className="inline-flex shrink-0" title={t('calendar.released')}>
        <CheckCheck aria-hidden className="size-3" />
      </span>
    )
  }

  const gaps = missing(release.readiness)
  const first = gaps[0]
  if (release.readiness.ready || first === undefined) {
    return (
      <span className="inline-flex shrink-0 text-good" title={t('calendar.ready')}>
        <Check aria-hidden className="size-3" />
      </span>
    )
  }

  const said = t('calendar.notReadyHint', {
    list: gaps.map((gap) => sayNeed(gap, profile.config, release, t)).join(', '),
  })

  return (
    <span className={cn('inline-flex shrink-0', TONE[urgency(daysLeft)])} title={said}>
      <NeedGlyph need={first} className="size-3" />
      <span className="sr-only">{said}</span>
    </span>
  )
}
