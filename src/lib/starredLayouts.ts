import { useSyncExternalStore } from 'react'

/*
 * The layouts a person stars (v0.135, FR-07 7.3): offered first in every
 * constructor, on every work - one memory for the application, as the
 * folded sections are. Storage that cannot be used stars nothing.
 */

const STORAGE_KEY = 'kilna.layouts.starred'
const CHANGED = 'kilna:layouts-starred'

function read(): string {
  try {
    return localStorage.getItem(STORAGE_KEY) ?? '[]'
  } catch {
    return '[]'
  }
}

function listed(raw: string): readonly string[] {
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

/** The starred layouts, and the switch that stars one. */
export function useStarredLayouts(): [readonly string[], (layout: string, star: boolean) => void] {
  const raw = useSyncExternalStore(subscribe, read, () => '[]')
  const starred = listed(raw)
  const setStarred = (layout: string, star: boolean) => {
    const rest = listed(read()).filter((one) => one !== layout)
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(star ? [...rest, layout] : rest))
    } catch {
      // Not remembered.
    }
    window.dispatchEvent(new Event(CHANGED))
  }
  return [starred, setStarred]
}
