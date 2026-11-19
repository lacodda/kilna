import { useSyncExternalStore } from 'react'

/**
 * How tightly the window lays out its rows and controls (v0.163): the theme's
 * own densities, set once on <html> so every field, button, list row and
 * table under it follows - dowel sizes them by `--row-control` and
 * `--row-cell`, and the attribute moves those. `regular` sets nothing.
 */
export type Density = 'compact' | 'regular' | 'comfortable'

const STORAGE_KEY = 'kilna.density'
export const DENSITIES: readonly Density[] = ['compact', 'regular', 'comfortable']

function storedDensity(): Density {
  let raw: string | null = null
  try {
    raw = localStorage.getItem(STORAGE_KEY)
  } catch {
    // Storage refused: the regular density, as on a first start.
  }
  return DENSITIES.includes(raw as Density) ? (raw as Density) : 'regular'
}

function applyDensity(density: Density): void {
  const root = document.documentElement
  if (density === 'regular') root.removeAttribute('data-density')
  else root.setAttribute('data-density', density)
}

/** Called once at startup, before React renders, so the first paint is sized. */
export function initDensity(): void {
  applyDensity(storedDensity())
}

const CHANGED = 'kilna:density'

function subscribe(onChange: () => void): () => void {
  window.addEventListener(CHANGED, onChange)
  return () => window.removeEventListener(CHANGED, onChange)
}

export function useDensity(): { density: Density; setDensity: (density: Density) => void } {
  const density = useSyncExternalStore(subscribe, storedDensity, (): Density => 'regular')

  const setDensity = (next: Density) => {
    try {
      localStorage.setItem(STORAGE_KEY, next)
    } catch {
      // Not kept past this window; still applied to it.
    }
    applyDensity(next)
    window.dispatchEvent(new Event(CHANGED))
  }

  return { density, setDensity }
}
