import { useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { useQueryClient } from '@tanstack/react-query'
import type { TFunction } from 'i18next'
import { warnUnreadyReleases } from '@/lib/api/releases'
import type { ProfileConfig, ScheduledRelease } from '@/lib/api/types'
import { today } from '@/lib/month'
import { tell, type Notice } from '@/lib/outward'
import { keys } from '@/lib/query/keys'
import { missing, sayNeed } from '@/lib/readiness'
import { useProfile } from '@/lib/useProfile'

/**
 * The sweep of the coming week, whoever runs it: the shell at start and when
 * the day turns, the calendar after every change (v0.107).
 *
 * The backend writes each warning once and hands back the releases due
 * tomorrow that this sweep warned about first. Those are said - in the window
 * or through the system - by whichever sweep found them. Said only by the
 * shell's, a release moved onto tomorrow from the calendar would have its
 * line written there and its news swallowed: the next sweep finds it already
 * written, and says nothing.
 */
export function useWeekSweep(): (day?: string) => Promise<void> {
  const { t } = useTranslation()
  const client = useQueryClient()
  const navigate = useNavigate()
  const { config } = useProfile()

  return useCallback(
    async (day = today()) => {
      const { standing, tomorrow } = await warnUnreadyReleases(day)
      if (standing > 0) void client.invalidateQueries({ queryKey: keys.journal })
      if (tomorrow.length > 0) tell(tomorrowNotice(tomorrow, config, t, navigate))
    },
    [client, config, navigate, t],
  )
}

/** What a sweep says about the releases due tomorrow and not ready. */
export function tomorrowNotice(
  tomorrow: readonly ScheduledRelease[],
  config: ProfileConfig,
  t: TFunction,
  go: (path: string) => unknown,
): Notice {
  const release = tomorrow[0]
  if (tomorrow.length === 1 && release !== undefined) {
    const gaps = missing(release.readiness).map((gap) => sayNeed(gap, config, release, t))
    return {
      title: t('journal.release.notReadyTomorrow', {
        title: release.work_title,
        date: release.scheduled_at,
      }),
      body: gaps.length === 0 ? undefined : t('calendar.notReadyHint', { list: gaps.join(', ') }),
      warn: true,
      open: { label: t('assistant.taskOpen'), run: () => go(`/works/${release.work_id}`) },
    }
  }
  return {
    title: t('outside.tomorrowMany', { count: tomorrow.length }),
    body: tomorrow.map((entry) => entry.work_title).join(', '),
    warn: true,
    open: { label: t('assistant.taskOpen'), run: () => go('/calendar') },
  }
}
