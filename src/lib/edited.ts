import type { QueryClient } from '@tanstack/react-query'
import i18n from '@/i18n'
import { lastUndoable, undoLast } from '@/lib/api'
import { say } from '@/lib/toast'

/**
 * The one place an edit offers to be taken back.
 *
 * The counterpart of `announceDeleted`, and the same bargain: nothing asks
 * before the fact, and one line afterwards says what happened with a way to
 * reverse it. `Ctrl+Z` does the same thing from anywhere; this is the offer for
 * the moment it is most likely to be wanted, when the person is still looking
 * at what they changed.
 *
 * What it offers to undo is read from the log rather than assumed from the
 * mutation that just ran. The two are almost always the same operation — but
 * "almost always" is how an undo button ends up reversing something else, and
 * the log is the only thing that knows what is actually last.
 */
export function announceEdited({
  client,
  message,
  refresh,
}: {
  client: QueryClient
  /** What changed, in the person's words: 'Title changed'. */
  message: string
  /** The query areas the edit disturbed — reused for the undo, which disturbs
   * exactly the same ones in the other direction. */
  refresh: readonly (readonly unknown[])[]
}) {
  void (async () => {
    const offer = await lastUndoable().catch(() => null)
    // Nothing to offer: the edit is not one of the kinds that can be taken back
    // (see `undo::reversible`), so the sentence stands on its own rather than
    // carrying a button that would fail.
    if (offer === null) {
      say.ok(message)
      return
    }

    say.undoable(message, i18n.t('toast.undo'), () => {
      undoLast(offer.operationId)
        .then(() => {
          void client.invalidateQueries()
          say.ok(i18n.t('undo.done', { what: i18n.t(offer.action, offer.params) }))
        })
        .catch((cause: unknown) => say.failedTo(i18n.t('undo.failed'), cause))
    })
  })()

  // The invalidation itself does not wait on the offer: the screen should
  // refresh the moment the edit lands, not a round trip later.
  for (const key of refresh) {
    void client.invalidateQueries({ queryKey: key })
  }
}
