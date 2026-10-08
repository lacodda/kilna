import { useTranslation } from 'react-i18next'
import type { Progress } from '@/lib/collections'
import { formatDay } from '@/lib/format'
import { cn } from '@/lib/utils'

/**
 * The goal in words: the day it is due by, and how much is still to come -
 * "Due Dec 1 · 5 to go". A day gone by with works still to come is said in
 * the colour of a deadline missed; one gone by with the collection full is
 * simply done, and says so.
 */
export function GoalLine({ progress, className }: { progress: Progress; className?: string }) {
  const { t } = useTranslation()
  const late = progress.daysLeft !== null && progress.daysLeft < 0 && !progress.met

  const parts: string[] = []
  if (progress.due !== null) {
    parts.push(
      t(late ? 'collections.goal.late' : 'collections.goal.due', {
        day: formatDay(progress.due),
      }),
    )
  }
  if (progress.left !== null) {
    parts.push(
      progress.met
        ? t('collections.goal.met')
        : t('collections.goal.left', { count: progress.left }),
    )
  }
  if (parts.length === 0) parts.push(t('collections.goal.none'))

  return <span className={cn(className, late && 'text-bad')}>{parts.join(' · ')}</span>
}
