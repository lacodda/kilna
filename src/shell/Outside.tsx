import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { defaultWindowIcon } from '@tauri-apps/api/app'
import { listen } from '@tauri-apps/api/event'
import { TrayIcon } from '@tauri-apps/api/tray'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { warnUnreadyReleases } from '@/lib/api/releases'
import { setTrayWords } from '@/lib/api/shell'
import type { ScheduledRelease } from '@/lib/api/types'
import { badgeDisc, badgedIcon, trayLine } from '@/lib/badge'
import { today } from '@/lib/month'
import { hearNoticeClicks, tell, type Notice } from '@/lib/outward'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { missing, sayNeed } from '@/lib/readiness'
import { say } from '@/lib/toast'
import { useProfile } from '@/lib/useProfile'
import type { TFunction } from 'i18next'
import type { ProfileConfig } from '@/lib/api/types'

/** The tray's id, as the backend builds it (`tray::ID`). */
const TRAY = 'kilna'

/** How often the window looks whether its day has turned. */
const DAY_CHECK_MS = 60_000

/** Whether there is a Tauri window around the page - not in a browser tab. */
const inTauri = () => '__TAURI_INTERNALS__' in window

/**
 * What the window says outside itself (v0.107): the tray's words and badge,
 * the number on the taskbar, the releases found unready the day before their
 * slot, and the click on a notification that brings back what it was about.
 *
 * Mounted once, at the shell. It draws nothing: everything it says is in the
 * system's places, which the page reaches through Tauri. The door back to the
 * window is the backend's (`tray.rs`); the words on it are the page's.
 */
export function Outside() {
  useTrayWords()
  useBadges()
  useDaySweep()
  useNoticeClicks()
  return null
}

/** The tray's menu in the window's language, sent again when it changes. */
function useTrayWords() {
  const { t, i18n } = useTranslation()
  const language = i18n.resolvedLanguage
  useEffect(() => {
    if (!inTauri()) return
    setTrayWords({ open: t('outside.trayOpen'), quit: t('outside.trayQuit') }).catch(
      () => undefined,
    )
  }, [t, language])
}

/**
 * The count of what is going, on the taskbar while the window is open and
 * on the tray while it is closed - with the tray's tooltip saying it in
 * words.
 */
function useBadges() {
  const { t } = useTranslation()
  const busy = useQuery(queries.activity()).data
  const [hidden, setHidden] = useState(false)

  useEffect(() => {
    const stop = listen<boolean>('app:background', ({ payload }) => setHidden(payload))
    return () => {
      void stop.then((unlisten) => unlisten())
    }
  }, [])

  useEffect(() => {
    if (!inTauri() || busy === undefined) return
    let gone = false
    const count = busy.running
    const draw = async () => {
      const window = getCurrentWindow()
      // Windows lays a picture over the taskbar button; the other systems
      // keep a count of their own. Each refuses what it does not do, and
      // neither refusal is worth a word.
      // A picture handed to the system is a resource the backend holds for
      // the page; each is let go once it has been given.
      const disc = count > 0 ? await badgeDisc(count) : null
      if (!gone) {
        await window.setOverlayIcon(disc ?? undefined).catch(() => undefined)
        await window.setBadgeCount(count > 0 ? count : undefined).catch(() => undefined)
      }
      await disc?.close().catch(() => undefined)
      if (!hidden || gone) return
      const tray = await TrayIcon.getById(TRAY)
      if (tray === null || gone) return
      await tray.setTooltip(trayLine(busy, t))
      const base = await defaultWindowIcon()
      const icon = base === null ? null : await badgedIcon(base, count)
      await base?.close().catch(() => undefined)
      if (icon !== null && !gone) await tray.setIcon(icon)
      await icon?.close().catch(() => undefined)
    }
    draw().catch(() => undefined)
    return () => {
      gone = true
    }
  }, [busy, hidden, t])
}

/**
 * The sweep of the coming week: at start, and again whenever the window's
 * day turns - a window left open, or waiting behind the tray, overnight
 * notices tomorrow's gaps on the new day. A release unready the day before
 * its slot is said outside the window, once (`warn_unready`).
 */
function useDaySweep() {
  const { t } = useTranslation()
  const client = useQueryClient()
  const navigate = useNavigate()
  const profile = useProfile()

  useEffect(() => {
    let day = today()
    const sweep = () => {
      // A sweep that fails says so: the week's gaps would otherwise go
      // unnoticed until the day, which is the one thing the sweep is for.
      warnUnreadyReleases(day)
        .then(({ standing, tomorrow }) => {
          if (standing > 0) void client.invalidateQueries({ queryKey: keys.journal })
          if (tomorrow.length > 0) tell(tomorrowNotice(tomorrow, profile.config, t, navigate))
        })
        .catch((cause: unknown) => say.failedTo(t('toast.sweepFailed'), cause))
    }
    sweep()
    const tick = setInterval(() => {
      const now = today()
      if (now === day) return
      day = now
      sweep()
    }, DAY_CHECK_MS)
    return () => clearInterval(tick)
    // A sweep per profile: the shell mounts this once for each (its key). A
    // change of the profile's words or of the language sweeps again, which
    // writes nothing new - every warning is written once.
  }, [profile.config, client, t, navigate])
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
    body: tomorrow.map((release) => release.work_title).join(', '),
    warn: true,
    open: { label: t('assistant.taskOpen'), run: () => go('/calendar') },
  }
}

/** A click on a notification opens what it was about. */
function useNoticeClicks() {
  useEffect(() => {
    if (!inTauri()) return
    let stop: (() => void) | null = null
    let gone = false
    hearNoticeClicks()
      .then((unlisten) => {
        if (gone) unlisten()
        else stop = unlisten
      })
      .catch(() => undefined)
    return () => {
      gone = true
      stop?.()
    }
  }, [])
}
