import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { onAction, sendNotification } from '@tauri-apps/plugin-notification'
import { say } from '@/lib/toast'

/**
 * Saying something to a person who may not be looking at the window (v0.107).
 *
 * In front, the window says it the way it says everything: a toast, with a
 * way to go and look. Not in front - hidden behind the tray, minimised, under
 * another program - a toast would come and go unseen, so the system says it
 * instead: a notification, which a click brings back to the window and onto
 * what it was about. The backend shows the window on that click
 * (`tray::show_main`); what to open is kept here, by the notification's id.
 *
 * Only what was *news* goes out: a task that ended, a release found unready
 * the day before its slot, a week that went over its budget - each written
 * into the history once, and said once.
 */
export interface Notice {
  /** The line itself: the toast's title, the notification's heading. */
  title: string
  /** More about it, under the line. */
  body?: string
  /** A warning rather than news. */
  warn?: boolean
  /** Where to go and look, with the toast's word for going there. */
  open?: { label: string; run: () => void }
}

/**
 * Where the window is, as the system says it.
 *
 * Not as the page believes it: a window hidden behind the tray still answers
 * `document.hasFocus()` with true and `visibilityState` with visible in
 * WebView2 - found by the v0.107 rehearsal, where every ending was toasted
 * into a window nobody could see. So the window's own focus events and the
 * backend's word on hiding it (`app:background`) are kept here once the shell
 * watches them; the page's answer stands in only where nothing watches - a
 * browser tab, a test.
 */
const presence = { watched: false, focused: false, hidden: false }

/** Start keeping where the window is. Once, at the shell. */
export async function watchPresence(): Promise<() => void> {
  const window = getCurrentWindow()
  presence.focused = await window.isFocused().catch(() => document.hasFocus())
  presence.watched = true
  const focus = await window.onFocusChanged(({ payload }) => {
    presence.focused = payload
  })
  const background = await listen<boolean>('app:background', ({ payload }) => {
    presence.hidden = payload
  })
  return () => {
    focus()
    background()
    presence.watched = false
    presence.hidden = false
  }
}

/** Whether the person is looking at the window right now. */
export function inFront(): boolean {
  if (presence.hidden) return false
  if (presence.watched) return presence.focused
  return document.visibilityState === 'visible' && document.hasFocus()
}

/** What a click on each notification opens, by the notification's id. */
const opens = new Map<number, () => void>()
let lastId = 0

/** Say it through the system, whatever the window is doing. */
export function notify(notice: Notice): void {
  lastId += 1
  const id = lastId
  if (notice.open !== undefined) opens.set(id, notice.open.run)
  try {
    sendNotification({ id, title: notice.title, body: notice.body })
  } catch {
    // No notifications here - a browser tab, a test. The history holds
    // what mattered; the toast would not have been seen either.
    opens.delete(id)
  }
}

/** Say it inside the window when it is in front, outside it when not. */
export function tell(notice: Notice): void {
  if (!inFront()) {
    notify(notice)
    return
  }
  if (notice.open !== undefined) {
    say.withAction(notice.title, notice.open.label, notice.open.run)
  } else if (notice.warn === true) {
    say.warn(notice.title, notice.body)
  } else {
    say.info(notice.title)
  }
}

/**
 * Hear clicks on the notifications this window raised. Once, at the shell;
 * the returned function stops listening.
 */
export async function hearNoticeClicks(): Promise<() => void> {
  const listener = await onAction((performed) => {
    if (performed.actionId !== 'tap') return
    const id = performed.notification?.id
    if (id === undefined) return
    opens.get(id)?.()
    opens.delete(id)
  })
  return () => {
    void listener.unregister()
  }
}
