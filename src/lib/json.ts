import type { JsonValue } from '@/lib/api/generated/serde_json/JsonValue'

/**
 * A JSON value as the text a box shows.
 *
 * A `meta`, a `cover`, a scene's `blocks` — every free-form map the backend
 * carries is typed `{ [key: string]: JsonValue }` rather than
 * `Record<string, string>`, because nothing on that side stops an older
 * export, a plugin or a hand-edited file from putting a number or a nested
 * object under one of its keys (ADR 0003). The window only ever writes
 * strings into these boxes; reading one back is this, once, rather than a
 * cast at every call site.
 */
export function textOf(value: JsonValue | undefined): string {
  if (value === undefined || value === null || typeof value === 'boolean') return ''
  if (typeof value === 'string') return value
  if (typeof value === 'number') return String(value)
  // An array or an object under a key meant for text: shown as JSON rather
  // than silently swallowed, so a stray value is at least visible.
  return JSON.stringify(value)
}

/** A free-form map read as strings — see {@link textOf}. */
export function textMap(map: Record<string, JsonValue>): Record<string, string> {
  return Object.fromEntries(Object.entries(map).map(([key, value]) => [key, textOf(value)]))
}
