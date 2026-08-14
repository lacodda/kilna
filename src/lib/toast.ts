import { toast } from 'sonner'
import { humanError } from '@/lib/errors'

/**
 * The app's whole vocabulary for saying something happened.
 *
 * Failures go through `humanError`, so no raw `String(cause)` reaches a person.
 * Kept deliberately small: a toast is for something that already happened and
 * needs no decision. Anything that needs an answer is a dialog.
 */
export const say = {
  ok: (message: string) => toast.success(message),

  /** A failure, with the sentence derived from whatever was thrown. */
  failed: (cause: unknown) => toast.error(humanError(cause)),

  /** A failure where the headline says what we were trying to do. */
  failedTo: (what: string, cause: unknown) =>
    toast.error(what, { description: humanError(cause) }),

  info: (message: string) => toast(message),

  warn: (message: string, description?: string) => toast.warning(message, { description }),

  /**
   * A completed action the person can take back.
   *
   * Deletion uses this rather than a confirm dialog: an undo costs one click
   * after the fact, a confirmation costs one click every time.
   */
  undoable: (message: string, undoLabel: string, onUndo: () => void) =>
    toast(message, { action: { label: undoLabel, onClick: onUndo } }),
}
