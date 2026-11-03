import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { defaultWindowIcon } from '@tauri-apps/api/app'
import { listen } from '@tauri-apps/api/event'
import { TrayIcon } from '@tauri-apps/api/tray'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { setTrayWords } from '@/lib/api/shell'
import { badgeDisc, badgedIcon, trayLine } from '@/lib/badge'
import { today } from '@/lib/month'
import { hearNoticeClicks, watchPresence } from '@/lib/outward'
import { queries } from '@/lib/query/queries'
import { say } from '@/lib/toast'
import { useWeekSweep } from '@/lib/weekSweep'

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
 * The sweep of the coming week at start, and again whenever the window's day
 * turns - a window left open, or waiting behind the tray, overnight notices
 * tomorrow's gaps on the new day (`lib/weekSweep`).
 */
function useDaySweep() {
  const { t } = useTranslation()
  const sweepWeek = useWeekSweep()

  useEffect(() => {
    let day = today()
    // A sweep that fails says so: the week's gaps would otherwise go
    // unnoticed until the day, which is the one thing the sweep is for.
    const sweep = () => {
      sweepWeek(day).catch((cause: unknown) => say.failedTo(t('toast.sweepFailed'), cause))
    }
    sweep()
    const tick = setInterval(() => {
      const now = today()
      if (now === day) return
      day = now
      sweep()
    }, DAY_CHECK_MS)
    return () => clearInterval(tick)
    // Per profile: the shell mounts this once for each (its key). A change of
    // the profile's words or of the language sweeps again, which writes
    // nothing new - every warning is written once.
  }, [sweepWeek, t])
}
/**
 * A click on a notification opens what it was about; and where the window is
 * - in front, behind another program, behind the tray - is kept from the
 * system's own word, which decides between a toast and a notification.
 */
function useNoticeClicks() {
  useEffect(() => {
    if (!inTauri()) return
    const stops: (() => void)[] = []
    let gone = false
    const keep = (unlisten: () => void) => {
      if (gone) unlisten()
      else stops.push(unlisten)
    }
    hearNoticeClicks()
      .then(keep)
      .catch(() => undefined)
    watchPresence()
      .then(keep)
      .catch(() => undefined)
    return () => {
      gone = true
      for (const stop of stops) stop()
    }
  }, [])
}
