import { useTranslation } from 'react-i18next'
import { Check, CheckCheck } from 'lucide-react'
import type { ScheduledRelease } from '@/lib/api/types'
import { daysBetween, missing, urgency, type Urgency } from '@/lib/readiness'
import { labelOf, useProfile, vocabularyOf } from '@/lib/useProfile'
import { Badge } from '@/components/ui/badge'

interface Props {
  release: ScheduledRelease
  /** Today, as the local date the days left are counted from. */
  today: string
}

// The colour belongs to the deadline, not to the gap: the same missing
// score is a calm note a month out and a red flag two days before the slot -
// the chip on the calendar reads it the same way (`ReadyMarks`).
const TONE: Record<Urgency, 'soft' | 'warn' | 'bad'> = {
  calm: 'soft',
  soon: 'warn',
  urgent: 'bad',
}

/**
 * Whether a release could go out, said on its row.
 *
 * In words where the calendar's chip has glyphs: a chip at eleven pixels has
 * room for a page and a gauge, a row of the work's own list has room to say
 * what is missing - and it is read by someone who can go and fill the gap
 * from the tab beside it. Two ticks it went out, one tick it is ready.
 */
export function ReleaseReadiness({ release, today }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()

  if (release.status === 'released') {
    return (
      <span className="inline-flex shrink-0 text-good" title={t('calendar.released')}>
        <CheckCheck aria-hidden className="size-3.5" />
        <span className="sr-only">{t('calendar.released')}</span>
      </span>
    )
  }

  const gaps = missing(release.readiness)
  if (gaps.length === 0) {
    return (
      <span className="inline-flex shrink-0 text-good" title={t('calendar.ready')}>
        <Check aria-hidden className="size-3.5" />
        <span className="sr-only">{t('calendar.ready')}</span>
      </span>
    )
  }

  const roles = vocabularyOf(profile.config, release.work_kind).version_roles
  const list = gaps
    .map((gap) => (gap === 'score' ? t('calendar.missingScore') : labelOf(roles, gap)))
    .join(', ')
  const daysLeft = release.scheduled_at === null ? null : daysBetween(today, release.scheduled_at)

  return (
    <Badge variant={TONE[urgency(daysLeft)]} className="max-w-64 min-w-0 shrink" title={list}>
      <span className="truncate">{t('calendar.notReadyHint', { list })}</span>
    </Badge>
  )
}
