import { useSyncExternalStore } from 'react'

/*
 * The selections pinned on the Notes screen (v0.145, FR-09 9.1): a kind and
 * a tag under a name - "Strategy documents" - one press away in the side
 * panel. Kept in this window's storage, as the catalogue's views are; storage
 * that cannot be read or written pins nothing and forgets quietly.
 */

/** A pinned selection of the notes. */
export interface Selection {
  kind: string
  tag: string
}

const STORAGE_KEY = 'kilna.notes.selections'
const CHANGED = 'kilna:selections'

function read(): string {
  try {
    return localStorage.getItem(STORAGE_KEY) ?? '[]'
  } catch {
    return '[]'
  }
}

/** The selections a stored value holds; anything else holds none. */
export function selectionsOf(raw: string): Selection[] {
  try {
    const list: unknown = JSON.parse(raw)
    if (!Array.isArray(list)) return []
    return list.filter(
      (one): one is Selection =>
        typeof one === 'object' &&
        one !== null &&
        typeof (one as Selection).kind === 'string' &&
        typeof (one as Selection).tag === 'string',
    )
  } catch {
    return []
  }
}

function subscribe(onChange: () => void): () => void {
  window.addEventListener(CHANGED, onChange)
  return () => window.removeEventListener(CHANGED, onChange)
}

function write(next: readonly Selection[]) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(next))
  } catch {
    // Not remembered: the pin lasts until the window closes.
  }
  window.dispatchEvent(new Event(CHANGED))
}

/** The pinned selections, and the switches that pin and unpin one. */
export function useSelections(): {
  selections: Selection[]
  pin: (one: Selection) => void
  unpin: (one: Selection) => void
} {
  const raw = useSyncExternalStore(subscribe, read, () => '[]')
  const selections = selectionsOf(raw)
  const same = (a: Selection, b: Selection) => a.kind === b.kind && a.tag === b.tag
  return {
    selections,
    pin: (one) => {
      const held = selectionsOf(read())
      if (!held.some((other) => same(other, one))) write([...held, one])
    },
    unpin: (one) => write(selectionsOf(read()).filter((other) => !same(other, one))),
  }
}
