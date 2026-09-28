import { invoke } from '@tauri-apps/api/core'
import type { ExportReport, ImportReport, PackageReport } from '@/lib/api/types'

// The workspace as files: exports, backups, imports.

/** Write a text the window composed to a path the person picked. The window
 * has no filesystem rights; the backend does the writing. */
export const writeTextFile = (path: string, text: string) =>
  invoke<string>('write_text_file', { path, text })
export const exportPackage = (workId: string, directory: string) =>
  invoke<PackageReport>('export_package', { workId, directory })
/** Whether a work has anything to pack: a board, or something written about a
 * release. A folder holding one nearly empty page is not worth offering. */
export const canExportPackage = (workId: string) =>
  invoke<boolean>('can_export_package', { workId })
export const exportMarkdown = (directory: string) =>
  invoke<ExportReport>('export_markdown', { directory })
export const backupWorkspace = (destination: string) =>
  invoke<string>('backup_workspace', { destination })
export const suggestedBackupName = () => invoke<string>('suggested_backup_name')
export const workspacePath = () => invoke<string>('workspace_path')
/** Where the application's own log is written: `logs/kilna.log` beside the
 * workspace. `null` before the log is open. */
export const logPath = () => invoke<string | null>('log_path')
/** Write a failure the window caught into that log: a release build has no
 * console, and the log is what a report of "it broke yesterday" can quote. */
export const logWindowError = (message: string) => invoke<void>('log_window_error', { message })
export const importLegacy = (source: string) => invoke<ImportReport>('import_legacy', { source })
