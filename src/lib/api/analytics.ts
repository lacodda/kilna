import { invoke } from '@tauri-apps/api/core'
import type { AnalyticsImport } from '@/lib/api/types'

// YouTube Studio's export read and kept (v0.160, FR-22 22.1 p.2).

/** Read an export against the space's releases; nothing is kept. */
export const readAnalytics = (path: string) => invoke<AnalyticsImport>('read_analytics', { path })

/** Keep the numbers of every row matched to a release; how many were. */
export const keepAnalytics = (path: string) => invoke<number>('keep_analytics', { path })
