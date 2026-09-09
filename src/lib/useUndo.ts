import { useQueryClient } from '@tanstack/react-query'
import { useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { lastUndoable, undoLast } from '@/lib/api'
import { say } from '@/lib/toast'

/**
 * Taking back the last thing that changed the workspace.
 *
 * What can be taken back is asked of the backend at the moment of the press,
 * never remembered here. Between an action and the keystroke a sweep, a second
 * window or the assistant may have written, and an offer built from a stale
 * memory would name one thing and reverse another — the backend refuses that,
 * and this asks so the refusal never has to happen.
 *
 * Deletion keeps its own toast (`announceDeleted`): it offers the undo where
 * the deletion happened, which is better than a keystroke for the gesture that
 * needs it most. This is the second way in, and the only way for an edit.
 */
export function useUndo(): { undo: () => void } {
  const client = useQueryClient()
  const { t } = useTranslation()

  const undo = useCallback(() => {
    void (async () => {
      const offer = await lastUndoable()
      if (offer === null) {
        say.ok(t('undo.nothing'))
        return
      }

      try {
        const taken = await undoLast(offer.operationId)
        // Everything, rather than the areas this particular undo disturbed: an
        // undo is rare, and a list of query keys per operation kind is a second
        // copy of what each command already knows — one that would go stale
        // quietly the first time a kind was added.
        await client.invalidateQueries()
        say.ok(t('undo.done', { what: t(taken.action, taken.params) }))
      } catch (cause: unknown) {
        say.failedTo(t('undo.failed'), cause)
      }
    })()
  }, [client, t])

  return { undo }
}
