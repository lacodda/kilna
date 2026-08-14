import i18n from '@/i18n'

// Mirrors the `Error` enum in src-tauri/src/error.rs. The backend sends both a
// stable `kind` and a message written for a human; `kind` is the part we are
// allowed to branch on, the message is the detail when we have nothing better.
export interface AppError {
  kind: string
  message: string
}

// Kinds we have a sentence for. Anything else falls through to the backend's
// own message rather than a lie about what went wrong.
const SPOKEN = new Set(['database', 'io', 'schemaTooNew', 'notFound', 'assistant'])

function isAppError(cause: unknown): cause is AppError {
  return (
    typeof cause === 'object' &&
    cause !== null &&
    typeof (cause as AppError).kind === 'string' &&
    typeof (cause as AppError).message === 'string'
  )
}

/**
 * A sentence to show a person, for any thrown value.
 *
 * Never returns `[object Object]` or an empty string: an unrecognised failure
 * still gets the generic sentence, because a blank toast is worse than a vague
 * one. This replaced `String(cause)` at every call site.
 */
export function humanError(cause: unknown): string {
  const t = i18n.t.bind(i18n)

  if (isAppError(cause)) {
    // A known kind gets our own wording, which is translatable and does not
    // leak SQL or file paths at someone who cannot act on them.
    if (SPOKEN.has(cause.kind)) return t(`error.${cause.kind}`)
    // `other` and anything newer than this build: the backend's message is
    // still a real sentence, so it beats a shrug.
    return cause.message.trim() === '' ? t('error.unknown') : cause.message
  }

  if (cause instanceof Error && cause.message.trim() !== '') return cause.message

  if (typeof cause === 'string' && cause.trim() !== '') return cause

  return t('error.unknown')
}

/** True when the failure is "it isn't there any more" — usually worth a refetch. */
export function isNotFound(cause: unknown): boolean {
  return isAppError(cause) && cause.kind === 'notFound'
}
