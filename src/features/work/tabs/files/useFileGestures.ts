import { useTranslation } from 'react-i18next'
import { save } from '@tauri-apps/plugin-dialog'
import { revealItemInDir } from '@tauri-apps/plugin-opener'
import { openMedia, saveFileCopy } from '@/lib/api/folders'
import type { FileEntry } from '@/lib/fileList'
import { say } from '@/lib/toast'

/**
 * What can be done with any file of a work, attached or on disk, that
 * changes nothing in the workspace (FR-14): open it in the program the
 * system opens it with, show it where it lies, copy its path, save a copy.
 *
 * In one place because the tab has two lists and a viewer, and each offers
 * these: three wordings of "saved" would be three places to fix one.
 */
export function useFileGestures() {
  const { t } = useTranslation()
  const failed = (cause: unknown) => say.failed(cause)

  return {
    openOutside: (entry: FileEntry) => {
      openMedia(entry.path).catch(failed)
    },
    reveal: (entry: FileEntry) => {
      revealItemInDir(entry.path).catch(failed)
    },
    copyPath: (entry: FileEntry) => {
      navigator.clipboard
        .writeText(entry.path)
        .then(() => say.ok(t('files.menu.pathCopied')), failed)
    },
    /** Ask where, and copy the file there. The system's own dialog asks
     *  about writing over a file that is there. */
    saveCopy: (entry: FileEntry) => {
      const run = async () => {
        const to = await save({ defaultPath: entry.name })
        if (to === null) return
        await saveFileCopy(entry.path, to)
        say.ok(t('files.menu.copySaved', { name: entry.name }))
      }
      run().catch((cause: unknown) => say.failedTo(t('files.menu.saveCopyFailed'), cause))
    },
  }
}
