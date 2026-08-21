import type { QueryClient } from '@tanstack/react-query'
import i18n from '@/i18n'
import { restoreDeletion } from '@/lib/api'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'

/**
 * The one place a deletion is announced.
 *
 * Every `delete_*` command returns the id of the trash entry it created, and
 * every call site hands it here: the person gets one sentence and one chance to
 * take it back, rather than a confirmation dialog before the fact. A dialog
 * costs a click every time; an undo costs a click only when it was a mistake.
 *
 * `refresh` is the query areas the deletion disturbed — the same prefixes the
 * mutation would invalidate, reused for the undo, because putting something
 * back changes exactly what taking it away did.
 */
export function announceDeleted({
  client,
  deletionId,
  deletionIds,
  message,
  refresh,
  onUndone,
}: {
  client: QueryClient
  /** One deletion to undo. Use `deletionIds` for a batch. */
  deletionId?: string
  /**
   * A batch, undone as a whole.
   *
   * Restoring is per entry, so a batch has to walk them — newest first,
   * because a work restored after its own version would find the version
   * already back and refuse. One toast for the lot: a toast per work would
   * push the earlier ones off the screen before they could be undone.
   */
  deletionIds?: readonly string[]
  /** What went, in the person's words: 'Winter road deleted'. */
  message: string
  refresh: readonly (readonly unknown[])[]
  /** Called after a successful undo — to reopen what was closed. */
  onUndone?: () => void
}) {
  // The journal goes with them: a deletion writes a line, and so does the undo
  // that takes it back.
  const invalidate = () => {
    for (const key of [...refresh, keys.deletions, keys.journal]) {
      void client.invalidateQueries({ queryKey: key })
    }
  }

  invalidate()

  const doomed = deletionIds ?? (deletionId === undefined ? [] : [deletionId])

  say.undoable(message, i18n.t('toast.undo'), () => {
    undoAll(doomed)
      .then(() => {
        invalidate()
        say.ok(i18n.t('trash.restored'))
        onUndone?.()
      })
      .catch((cause: unknown) => say.failedTo(i18n.t('trash.restoreFailed'), cause))
  })
}

/** Restore a batch in reverse order, so children come back before their parents. */
async function undoAll(deletionIds: readonly string[]): Promise<void> {
  for (const id of [...deletionIds].reverse()) {
    await restoreDeletion(id)
  }
}
