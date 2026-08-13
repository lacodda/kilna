import { useEffect, useState } from 'react'

// 'system' follows the OS; an explicit choice pins a class on <html>.
export type Theme = 'system' | 'light' | 'dark'

const STORAGE_KEY = 'kilna.theme'
const THEMES: readonly Theme[] = ['system', 'light', 'dark']

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

export function useTheme(): { theme: Theme; setTheme: (theme: Theme) => void } {
  const [theme, setThemeState] = useState<Theme>(storedTheme)

  useEffect(() => {
    applyTheme(theme)
  }, [theme])

  const setTheme = (next: Theme) => {
    localStorage.setItem(STORAGE_KEY, next)
    setThemeState(next)
  }

  return { theme, setTheme }
}

// The header button cycles through the three states in a fixed order.
export function nextTheme(current: Theme): Theme {
  const index = THEMES.indexOf(current)
  return THEMES[(index + 1) % THEMES.length] ?? 'system'
}
