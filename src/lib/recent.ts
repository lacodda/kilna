/**
 * The works you have opened lately, newest first.
 *
 * A list of everything answers "what exists"; this answers "what am I working
 * on", which is a different and much shorter question. In practice a person
 * returns to the same three or four works for days and then never opens them
 * again, so the useful list is short and recency-ordered rather than complete.
 *
 * It is kept in the browser's storage rather than in the database on purpose:
 * it is not a fact about the work, it is a fact about this machine's use of
 * it, and it must not travel to another device when the two are merged in a
 * later major.
 */

/** A work worth offering again. */
export interface Recent {
  id: string
  title: string
}

/** How many are kept. Past a handful the list stops being recognisable at a
 * glance, which is the only thing it is for — anything longer is what the
 * search box is. */
export const RECENT_LIMIT = 6

const RECENT_KEY = 'kilna.recent'

/** Just enough of `localStorage` to be handed a fake one, as elsewhere. */
export interface RecentStore {
  getItem: (key: string) => string | null
  setItem: (key: string, value: string) => void
}

/**
 * Put a work at the front of the list.
 *
 * Opening something already in the list moves it to the front rather than
 * adding it twice, so the list stays a set ordered by recency. The title is
 * stored alongside the id because the list has to be drawn before anything is
 * fetched — and a renamed work simply shows its old name until it is opened
 * again, which is cheaper than a lookup for every entry on every open.
 */
export function remember(list: Recent[], entry: Recent): Recent[] {
  const rest = list.filter((held) => held.id !== entry.id)
  return [entry, ...rest].slice(0, RECENT_LIMIT)
}

/** Drop a work from the list — what deleting one has to do, so the list never
 * offers something that is gone. */
export function forget(list: Recent[], id: string): Recent[] {
  return list.filter((held) => held.id !== id)
}

export function loadRecent(store: RecentStore = localStorage): Recent[] {
  try {
    const raw = store.getItem(RECENT_KEY)
    if (raw === null) return []

    const parsed: unknown = JSON.parse(raw)
    if (!Array.isArray(parsed)) return []

    // Filtered rather than rejected wholesale: one malformed entry should cost
    // that entry, not the whole list.
    return parsed.filter(isRecent).slice(0, RECENT_LIMIT)
  } catch {
    return []
  }
}

export function saveRecent(list: Recent[], store: RecentStore = localStorage): void {
  try {
    store.setItem(RECENT_KEY, JSON.stringify(list.slice(0, RECENT_LIMIT)))
  } catch {
    // Storage full or blocked: the list is a convenience, not a record.
  }
}

function isRecent(value: unknown): value is Recent {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) return false
  const candidate = value as Record<string, unknown>
  return (
    typeof candidate.id === 'string' &&
    candidate.id !== '' &&
    typeof candidate.title === 'string'
  )
}

/**
 * Record that a work was opened.
 *
 * Reads the stored list, moves this work to the front and writes it back — the
 * whole exchange, so a caller cannot hold a stale copy and overwrite a newer
 * entry with it. Cheap enough to do on every open: the list is six entries.
 */
export function noteOpened(entry: Recent, store: RecentStore = localStorage): void {
  saveRecent(remember(loadRecent(store), entry), store)
}

/** Drop a work that no longer exists, so the list never offers it. */
export function noteDeleted(id: string, store: RecentStore = localStorage): void {
  saveRecent(forget(loadRecent(store), id), store)
}
