import { useSyncExternalStore } from 'react'

// 'system' follows the OS; an explicit choice pins a class on <html>.
export type Theme = 'system' | 'light' | 'dark'

const STORAGE_KEY = 'kilna.theme'
export const THEMES: readonly Theme[] = ['system', 'light', 'dark']

function storedTheme(): Theme {
  const raw = localStorage.getItem(STORAGE_KEY)
  return THEMES.includes(raw as Theme) ? (raw as Theme) : 'system'
}

function applyTheme(theme: Theme): void {
  const root = document.documentElement
  root.classList.toggle('light', theme === 'light')
  root.classList.toggle('dark', theme === 'dark')
}

// Called once at startup, before React renders, so the first paint is themed.
export function initTheme(): void {
  applyTheme(storedTheme())
}

/**
 * One stored value, read by every place that shows or changes the theme - the
 * rail, Settings, the palette - and announced to all of them when it changes,
 * the way the language is. Each used to keep a copy of its own, so a theme
 * picked in Settings left the rail naming the old one.
 */
const CHANGED = 'kilna:theme'

function subscribe(onChange: () => void): () => void {
  window.addEventListener(CHANGED, onChange)
  return () => window.removeEventListener(CHANGED, onChange)
}

export function useTheme(): { theme: Theme; setTheme: (theme: Theme) => void } {
  const theme = useSyncExternalStore(subscribe, storedTheme, (): Theme => 'system')

  const setTheme = (next: Theme) => {
    localStorage.setItem(STORAGE_KEY, next)
    // Applied here rather than in an effect of whoever asked: the palette asks
    // and closes in the same moment, and the effect of a component on its way
    // out never runs.
    applyTheme(next)
    window.dispatchEvent(new Event(CHANGED))
  }

  return { theme, setTheme }
}

// The header button cycles through the three states in a fixed order.
export function nextTheme(current: Theme): Theme {
  const index = THEMES.indexOf(current)
  return THEMES[(index + 1) % THEMES.length] ?? 'system'
}
