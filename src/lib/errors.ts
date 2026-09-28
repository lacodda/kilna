import i18n from '@/i18n'

import type { JsonValue } from '@/lib/api/generated/serde_json/JsonValue'
import type { Reason } from '@/lib/api/generated/Reason'

// Mirrors the `Error` enum in src-tauri/src/error.rs. Its own wire shape is
// hand-written, not generated: the backend sends a stable `kind` and a
// message written for a human; `kind` is the part we are allowed to branch
// on. A refusal also carries a `code` and the `params` its sentence needs -
// the backend never writes the sentence a person reads, because the person
// may be reading Russian (ADR 0041).
export interface AppError {
  kind: string
  message: string
  code?: string
  params?: Record<string, JsonValue>
}

/**
 * A reason, generated from `crate::error::Reason` (ADR 0003): a key of the
 * locale and the values its sentence needs - `refusal.<code>`, `error.<kind>`
 * or `skip.<why>`. Why a batch passed an item over, or one problem among
 * several.
 */
export type { Reason }

// Kinds we have a sentence for. Anything else falls through to the backend's
// own message rather than a lie about what went wrong.
const SPOKEN = new Set([
  'database',
  'io',
  'schemaTooNew',
  'notFound',
  'notRestorable',
  'layoutStale',
  'assistant',
  'alreadyRunning',
  'busy',
  'frozen',
  'internal',
])

function isAppError(cause: unknown): cause is AppError {
  return (
    typeof cause === 'object' &&
    cause !== null &&
    typeof (cause as AppError).kind === 'string' &&
    typeof (cause as AppError).message === 'string'
  )
}

function isReason(value: unknown): value is Reason {
  return (
    typeof value === 'object' &&
    value !== null &&
    typeof (value as Reason).key === 'string' &&
    typeof (value as Reason).params === 'object'
  )
}

/**
 * The values a sentence interpolates, each said in the window's language: a
 * reason inside a reason in its own words, a list of reasons one after
 * another. A word of the profile (`{ en, ru }`) is left to the i18n formatter,
 * which says it in the language of the sentence.
 */
function spoken(params: Record<string, unknown>): Record<string, unknown> {
  const out: Record<string, unknown> = {}
  for (const [name, value] of Object.entries(params)) {
    if (isReason(value)) out[name] = sayReason(value)
    else if (Array.isArray(value))
      out[name] = value.map((item) => (isReason(item) ? sayReason(item) : String(item))).join('; ')
    else out[name] = value
  }
  return out
}

/** A reason, said in the window's language. Its key when the locale lacks it. */
export function sayReason(reason: Reason): string {
  if (!i18n.exists(reason.key)) return reason.key
  return i18n.t(reason.key, spoken(reason.params))
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
    // A refusal is said from its code, in the window's language. A code this
    // build has no sentence for - a backend newer than the window - keeps
    // the backend's English, which is still a real sentence.
    if (cause.kind === 'refused' && typeof cause.code === 'string') {
      const said = sayReason({ key: `refusal.${cause.code}`, params: cause.params ?? {} })
      if (said !== `refusal.${cause.code}`) return said
    }
    // A known kind gets our own wording, which is translatable and does not
    // leak SQL or file paths at someone who cannot act on them.
    if (SPOKEN.has(cause.kind)) return t(`error.${cause.kind}`)
    // Anything newer than this build: the backend's message is still a real
    // sentence, so it beats a shrug.
    return cause.message.trim() === '' ? t('error.unknown') : cause.message
  }

  if (cause instanceof Error && cause.message.trim() !== '') return cause.message

  if (typeof cause === 'string' && cause.trim() !== '') return cause

  return t('error.unknown')
}
