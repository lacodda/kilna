import { invoke } from '@tauri-apps/api/core'
import type { ExportReport, ImportReport, PackageReport } from '@/lib/api/types'

// The workspace as files: exports, backups, imports.

/** Write a text the window composed to a path the person picked. The window
 * has no filesystem rights; the backend does the writing. */
export const writeTextFile = (path: string, text: string) =>
  invoke<string>('write_text_file', { path, text })
export const exportPackage = (workId: string, directory: string) =>
  invoke<PackageReport>('export_package', { workId, directory })
export const exportMarkdown = (directory: string) =>
  invoke<ExportReport>('export_markdown', { directory })
export const backupWorkspace = (destination: string) =>
  invoke<string>('backup_workspace', { destination })
export const suggestedBackupName = () => invoke<string>('suggested_backup_name')
export const workspacePath = () => invoke<string>('workspace_path')
export const importLegacy = (source: string) => invoke<ImportReport>('import_legacy', { source })
