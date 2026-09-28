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

/** The two widths, as grid track sizes. Both are the theme's, the widths every
 *  rail of the line shares: the compact one is dowel's `NavRail collapsed` - a
 *  36px entry with air on each side. It was kilna's own 52px until the rail
 *  moved onto NavRail, and 52 is a pixel short of an entry, its inset and the
 *  rail's border, which would have drawn a sideways bar under the icons. */
export const RAIL_WIDTH: Record<RailWidth, string> = {
  full: 'var(--spacing-rail)',
  compact: 'var(--spacing-rail-compact)',
}

/** What is stored, or the full menu — an absent or broken value is not an error. */
function storedRail(): RailWidth {
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
