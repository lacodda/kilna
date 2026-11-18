import { useSyncExternalStore } from 'react'

/*
 * Which sections of the constructors are folded (v0.135, FR-07 7.3): one
 * memory for the whole application - a person who folds the mark on one
 * cover does not want it open on the next - kept in this window's storage
 * and announced to every section that shows it. Storage that cannot be read
 * or written (a private window) folds nothing and forgets quietly.
 */

const STORAGE_KEY = 'kilna.folded'
const CHANGED = 'kilna:folded'

function read(): string {
  try {
    return localStorage.getItem(STORAGE_KEY) ?? '[]'
  } catch {
    return '[]'
  }
}

function folded(raw: string): readonly string[] {
  try {
    const list: unknown = JSON.parse(raw)
    return Array.isArray(list) ? list.filter((one): one is string => typeof one === 'string') : []
  } catch {
    return []
  }
}

function subscribe(onChange: () => void): () => void {
  window.addEventListener(CHANGED, onChange)
  return () => window.removeEventListener(CHANGED, onChange)
}

/** Whether the section `id` is folded, and the switch that folds it. */
export function useFolded(id: string): [boolean, (fold: boolean) => void] {
  // The raw string is the snapshot: stable between changes, so React does
  // not see a new value on every read.
  const raw = useSyncExternalStore(subscribe, read, () => '[]')
  const isFolded = folded(raw).includes(id)
  const setFolded = (fold: boolean) => {
    const rest = folded(read()).filter((one) => one !== id)
    const next = fold ? [...rest, id] : rest
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(next))
    } catch {
      // Not remembered: the section still folds for as long as it is shown.
    }
    window.dispatchEvent(new Event(CHANGED))
  }
  return [isFolded, setFolded]
}
