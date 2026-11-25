import { useSyncExternalStore } from 'react'

/**
 * How much contrast the window is drawn with (v0.167): the theme's own mode
 * of more contrast, which follows the system unless a class on <html> pins it
 * on or off - the way the theme is pinned. Under it the dim ink clears AAA on
 * every ground, the faint one AA on the tinted fills too, and a hairline is a
 * boundary. `system` sets nothing and leaves the answer to the machine.
 */
export type Contrast = 'system' | 'more' | 'normal'

const STORAGE_KEY = 'kilna.contrast'
export const CONTRASTS: readonly Contrast[] = ['system', 'more', 'normal']
/** The class the theme reads for each pinned answer. */
const CLASSES = { more: 'contrast-more', normal: 'contrast-normal' } as const

function storedContrast(): Contrast {
  let raw: string | null = null
  try {
    raw = localStorage.getItem(STORAGE_KEY)
  } catch {
    // Storage refused: the system's answer, as on a first start.
  }
  return CONTRASTS.includes(raw as Contrast) ? (raw as Contrast) : 'system'
}

function applyContrast(contrast: Contrast): void {
  const root = document.documentElement
  root.classList.toggle(CLASSES.more, contrast === 'more')
  root.classList.toggle(CLASSES.normal, contrast === 'normal')
}

/** Called once at startup, before React renders, so the first paint is right. */
export function initContrast(): void {
  applyContrast(storedContrast())
}

const CHANGED = 'kilna:contrast'

function subscribe(onChange: () => void): () => void {
  window.addEventListener(CHANGED, onChange)
  return () => window.removeEventListener(CHANGED, onChange)
}

export function useContrast(): { contrast: Contrast; setContrast: (contrast: Contrast) => void } {
  const contrast = useSyncExternalStore(subscribe, storedContrast, (): Contrast => 'system')

  const setContrast = (next: Contrast) => {
    try {
      localStorage.setItem(STORAGE_KEY, next)
    } catch {
      // Not kept past this window; still applied to it.
    }
    applyContrast(next)
    window.dispatchEvent(new Event(CHANGED))
  }

  return { contrast, setContrast }
}
