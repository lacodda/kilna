import { createToastManager } from '@/components/ui/toast'
import { humanError } from '@/lib/errors'

/**
 * The manager behind every toast in the app.
 *
 * Created outside React because `say` is called from plain functions - a
 * mutation's `onError`, a catch block, an event handler - none of which can
 * reach a hook. `ToastProvider` is handed this same instance, so what is
 * raised here is what the viewport renders.
 */
export const toastManager = createToastManager()

/**
 * The app's whole vocabulary for saying something happened.
 *
 * Failures go through `humanError`, so no raw `String(cause)` reaches a person.
 * Kept deliberately small: a toast is for something that already happened and
 * needs no decision. Anything that needs an answer is a dialog.
 */
export const say = {
  ok: (message: string) => toastManager.add({ type: 'success', title: message }),

  /** A failure, with the sentence derived from whatever was thrown. */
  failed: (cause: unknown) => toastManager.add({ type: 'error', title: humanError(cause) }),

  /** A failure where the headline says what we were trying to do. */
  failedTo: (what: string, cause: unknown) =>
    toastManager.add({ type: 'error', title: what, description: humanError(cause) }),

  info: (message: string) => toastManager.add({ title: message }),

  warn: (message: string, description?: string) =>
    toastManager.add({ type: 'warning', title: message, description }),

  /**
   * A completed action the person can take back.
   *
   * Deletion uses this rather than a confirm dialog: an undo costs one click
   * after the fact, a confirmation costs one click every time.
   */
  undoable: (message: string, undoLabel: string, onUndo: () => void) =>
    toastManager.add({ title: message, actionProps: { children: undoLabel, onClick: onUndo } }),

  /**
   * Something finished elsewhere, with a way to go and look at it.
   *
   * The same shape as `undoable` and deliberately not the same name: one takes
   * an action back, the other goes to where it landed, and a reader of the call
   * site should not have to guess which.
   */
  withAction: (message: string, label: string, onAct: () => void) =>
    toastManager.add({ title: message, actionProps: { children: label, onClick: onAct } }),
}
