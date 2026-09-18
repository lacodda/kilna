import { useEffect, useState } from 'react'
import { DEFAULT_TAB, DEFAULT_TAB_CHOICES, type Tab } from '@/components/card/tabs'

/**
 * What the work card shows, as this machine prefers it.
 *
 * On the machine and not on the profile, for the same reason the theme is: it
 * answers "what do I want to look at", not "what does this craft consist of".
 * The craft's answer to the second is `work_meta_fields`, and hiding the strip
 * does not take a single field away — the values stay, the editor still edits
 * them, exports still carry them. This only stops drawing them.
 *
 * A record rather than one boolean, because the next thing someone wants gone
 * from the card will arrive, and a second module for a second switch is how a
 * preferences screen becomes six unrelated keys.
 */
export interface CardView {
  /** The row of craft fields under the tags: LANGUAGE, MOOD, TEMPO… */
  metaStrip: boolean
  /** The tab a card opens on when the address names none. Overview by
   *  default; the owner who lives in Versions sets Versions. */
  defaultTab: Tab
}

const STORAGE_KEY = 'kilna.card.view'

const DEFAULTS: CardView = {
  // Off by default. The strip was on for everyone until now and the owner's
  // verdict on it was "I see no use in these" — a row of eight truncated
  // values is a poor way to read fields the Overview tab already lays out.
  // Whoever wants it back turns it on and it stays on.
  metaStrip: false,
  defaultTab: DEFAULT_TAB,
}

/** What is stored, or the defaults — a broken or absent value is not an error. */
export function storedCardView(): CardView {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (raw === null) return DEFAULTS
    const parsed: unknown = JSON.parse(raw)
    if (typeof parsed !== 'object' || parsed === null) return DEFAULTS
    const held = parsed as Partial<Record<keyof CardView, unknown>>
    return {
      metaStrip:
        typeof held.metaStrip === 'boolean' ? held.metaStrip : DEFAULTS.metaStrip,
      // A tab that no longer exists, or one a kind may lack, falls back rather
      // than opening on nothing.
      defaultTab: (DEFAULT_TAB_CHOICES as readonly string[]).includes(
        held.defaultTab as string,
      )
        ? (held.defaultTab as Tab)
        : DEFAULTS.defaultTab,
    }
  } catch {
    // A private window, or storage the machine refuses. The card is worth
    // drawing either way.
    return DEFAULTS
  }
}

/**
 * The preference, and a way to change it.
 *
 * A window event rather than a context: the switch lives on the Settings
 * screen and the strip it governs is on a different route, so the two are
 * never in one tree. Storage events do not fire in the tab that wrote them,
 * which is exactly the tab that has to redraw.
 */
const CHANGED = 'kilna:cardview'

export function useCardView(): {
  view: CardView
  setCardView: (next: Partial<CardView>) => void
} {
  const [view, setView] = useState<CardView>(storedCardView)

  useEffect(() => {
    const listen = () => setView(storedCardView())
    window.addEventListener(CHANGED, listen)
    return () => window.removeEventListener(CHANGED, listen)
  }, [])

  const setCardView = (next: Partial<CardView>) => {
    const merged = { ...storedCardView(), ...next }
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(merged))
    } catch {
      // Not stored is not "not applied": the screen still follows the choice
      // for as long as the window is open.
    }
    setView(merged)
    window.dispatchEvent(new Event(CHANGED))
  }

  return { view, setCardView }
}
