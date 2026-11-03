import { invoke } from '@tauri-apps/api/core'
import type { Activity, TrayWords } from '@/lib/api/types'

// The application around the window: how busy it is, and the tray's words
// (v0.107).

/** Runs and plugin commands going, tasks waiting for a slot. */
export const activity = () => invoke<Activity>('activity')
/** What the tray's menu says, in this window's language. */
export const setTrayWords = (said: TrayWords) => invoke<void>('set_tray_words', { said })
