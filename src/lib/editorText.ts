import { useSyncExternalStore } from 'react'

/**
 * How a version's text is set in the editor (v0.163): the size of every
 * body, and the face a sung text is set in. A style prompt and any other
 * plain text that is not sung stays monospace whatever is chosen - it is
 * pasted into a generator, and its spaces and line breaks are the point.
 * Kept on <html> as attributes the stylesheet reads, so the editor's mirror
 * and its text change together and the marks stay on their letters.
 */
export type EditorFont = 'mono' | 'sans'
export type EditorSize = 'regular' | 'large'

export const EDITOR_FONTS: readonly EditorFont[] = ['mono', 'sans']
export const EDITOR_SIZES: readonly EditorSize[] = ['regular', 'large']

const FONT_KEY = 'kilna.editorFont'
const SIZE_KEY = 'kilna.editorSize'
const CHANGED = 'kilna:editor-text'

function stored<T extends string>(key: string, allowed: readonly T[], fallback: T): T {
  let raw: string | null = null
  try {
    raw = localStorage.getItem(key)
  } catch {
    // Storage refused: what a first start shows.
  }
  return allowed.includes(raw as T) ? (raw as T) : fallback
}

const storedFont = () => stored(FONT_KEY, EDITOR_FONTS, 'mono')
const storedSize = () => stored(SIZE_KEY, EDITOR_SIZES, 'regular')

function apply(font: EditorFont, size: EditorSize): void {
  const root = document.documentElement
  if (font === 'mono') root.removeAttribute('data-editor-font')
  else root.setAttribute('data-editor-font', font)
  if (size === 'regular') root.removeAttribute('data-editor-size')
  else root.setAttribute('data-editor-size', size)
}

/** Called once at startup, before React renders. */
export function initEditorText(): void {
  apply(storedFont(), storedSize())
}

function subscribe(onChange: () => void): () => void {
  window.addEventListener(CHANGED, onChange)
  return () => window.removeEventListener(CHANGED, onChange)
}

function keep(key: string, value: string) {
  try {
    localStorage.setItem(key, value)
  } catch {
    // Not kept past this window; still applied to it.
  }
}

export function useEditorText(): {
  font: EditorFont
  size: EditorSize
  setFont: (font: EditorFont) => void
  setSize: (size: EditorSize) => void
} {
  const font = useSyncExternalStore(subscribe, storedFont, (): EditorFont => 'mono')
  const size = useSyncExternalStore(subscribe, storedSize, (): EditorSize => 'regular')

  return {
    font,
    size,
    setFont: (next) => {
      keep(FONT_KEY, next)
      apply(next, storedSize())
      window.dispatchEvent(new Event(CHANGED))
    },
    setSize: (next) => {
      keep(SIZE_KEY, next)
      apply(storedFont(), next)
      window.dispatchEvent(new Event(CHANGED))
    },
  }
}
