import { invoke } from '@tauri-apps/api/core'
import type { Compacted, Examination, HistoryKeeping, KeptBackup, Storage } from '@/lib/api/types'

// Looking after the open space: its kept copies, its size, an examination of
// it, and how long its history is kept (v0.114).

/** The copies kilna keeps of the open space, newest first. */
export const keptBackups = () => invoke<KeptBackup[]>('kept_backups')
/** Put a backup back over the open space; answers with where what stood
 * before was kept. */
export const restoreBackup = (path: string) => invoke<string>('restore_backup', { path })
/** How much room the open space takes, by what takes it. */
export const storageReport = () => invoke<Storage>('storage_report')
/** Rewrite the database without the room it no longer uses. */
export const compactWorkspace = () => invoke<Compacted>('compact_workspace')
/** Examine the open space. Reads only. */
export const examineWorkspace = () => invoke<Examination>('examine_workspace')
/** How long the open space keeps its history, and where it goes after. */
export const historyKeeping = () => invoke<HistoryKeeping>('history_keeping')
/** Say how long the open space keeps its history: days, or 0 for good. */
export const setHistoryDays = (days: number) => invoke<HistoryKeeping>('set_history_days', { days })
