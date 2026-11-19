import { invoke } from '@tauri-apps/api/core'
import type { Diagnostics, UpdateCheck } from '@/lib/api/types'

// What to send when something went wrong, and whether a newer kilna is out
// (v0.162).

export const diagnostics = () => invoke<Diagnostics>('diagnostics')

/** The report as text to paste into an issue. */
export const diagnosticsText = () => invoke<string>('diagnostics_text')

/** Ask the project's releases whether a newer kilna is out. */
export const checkUpdate = () => invoke<UpdateCheck>('check_update')
