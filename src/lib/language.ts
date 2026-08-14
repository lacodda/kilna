import { useSyncExternalStore } from 'react'
import i18n from '@/i18n'

// The languages the app ships. `system` follows the OS, an explicit choice pins.
export type Language = 'system' | 'en' | 'ru'

export const LANGUAGES: readonly Language[] = ['system', 'en', 'ru']

/** The locales that actually exist as files — what `system` may resolve to. */
const SUPPORTED = new Set(['en', 'ru'])
const STORAGE_KEY = 'kilna.language'
const FALLBACK = 'en'

export function storedLanguage(): Language {
  const raw = localStorage.getItem(STORAGE_KEY)
  return LANGUAGES.includes(raw as Language) ? (raw as Language) : 'system'
}

/**
 * The locale a choice resolves to.
 *
 * `navigator.language` is a full tag like `ru-RU`; only the base language is
 * meaningful here, since kilna does not carry regional variants.
 */
export function resolveLanguage(choice: Language): string {
  if (choice !== 'system') return choice

  for (const tag of navigator.languages ?? [navigator.language]) {
    const base = tag.split('-')[0]?.toLowerCase()
    if (base !== undefined && SUPPORTED.has(base)) return base
  }
  return FALLBACK
}

/**
 * Called once at startup, before React renders.
 *
 * Without this the first paint would be in whatever i18n was initialised with,
 * and the window would visibly change language a moment later.
 */
export function initLanguage(): void {
  void i18n.changeLanguage(resolveLanguage(storedLanguage()))

  // `<html lang>` is what hyphenation and screen readers go by, so it has to
  // follow the interface rather than stay at whatever index.html declared.
  i18n.on('languageChanged', (language) => {
    document.documentElement.lang = language
  })
  document.documentElement.lang = i18n.resolvedLanguage ?? FALLBACK
}

// i18next is the source of truth for "what language is showing"; the stored
// choice is a separate question ('system' is a choice, not a language).
const subscribe = (onChange: () => void) => {
  i18n.on('languageChanged', onChange)
  return () => i18n.off('languageChanged', onChange)
}

export function useLanguage(): {
  language: Language
  resolved: string
  setLanguage: (language: Language) => void
} {
  // Re-render on every language change, wherever it was triggered from.
  const resolved = useSyncExternalStore(
    subscribe,
    () => i18n.resolvedLanguage ?? FALLBACK,
    () => FALLBACK,
  )

  const setLanguage = (next: Language) => {
    localStorage.setItem(STORAGE_KEY, next)
    void i18n.changeLanguage(resolveLanguage(next))
  }

  return { language: storedLanguage(), resolved, setLanguage }
}

// The footer button cycles through the three states in a fixed order, the way
// the theme button does.
export function nextLanguage(current: Language): Language {
  const index = LANGUAGES.indexOf(current)
  return LANGUAGES[(index + 1) % LANGUAGES.length] ?? 'system'
}
