import { useEffect, useState } from 'react'

/**
 * Whether the main menu is drawn in full or as a column of icons.
 *
 * On the machine and not on the profile, for the reason the theme and the
 * card view are: how much room the menu takes is a habit of the person at this
 * screen, not a fact of the craft. A laptop wants the icons; a wide monitor
 * can afford the words.
 */
export type RailWidth = 'full' | 'compact'

const STORAGE_KEY = 'kilna.rail'

/** The two widths, as grid track sizes. The full one is the theme's, the
 *  width every rail of the line shares; the compact one is an icon and its
 *  padding, which no other product has yet. */
export const RAIL_WIDTH: Record<RailWidth, string> = {
  full: 'var(--spacing-rail)',
  compact: '52px',
}

/** What is stored, or the full menu — an absent or broken value is not an error. */
export function storedRail(): RailWidth {
  try {
    return localStorage.getItem(STORAGE_KEY) === 'compact' ? 'compact' : 'full'
  } catch {
    // A private window, or storage the machine refuses. The menu is drawn
    // either way.
    return 'full'
  }
}

/**
 * A window event rather than a context, the same bargain `useCardView` makes:
 * the handle is in the title bar and the menu it folds is a sibling of it, and
 * the grid that sizes them is their parent. All three read one stored value.
 */
const CHANGED = 'kilna:rail'

export function useRail(): { rail: RailWidth; toggle: () => void } {
  const [rail, setRail] = useState<RailWidth>(storedRail)

  useEffect(() => {
    const listen = () => setRail(storedRail())
    window.addEventListener(CHANGED, listen)
    return () => window.removeEventListener(CHANGED, listen)
  }, [])

  const toggle = () => {
    const next: RailWidth = storedRail() === 'compact' ? 'full' : 'compact'
    try {
      localStorage.setItem(STORAGE_KEY, next)
    } catch {
      // Not stored is not "not applied": the window follows the choice for as
      // long as it is open.
    }
    setRail(next)
    window.dispatchEvent(new Event(CHANGED))
  }

  return { rail, toggle }
}
