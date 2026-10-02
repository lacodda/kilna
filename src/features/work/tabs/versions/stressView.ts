import { useCallback, useState } from 'react'

/*
 * Whether a sung text is drawn with an accent over each stressed vowel
 * (ADR 0053), as this machine prefers it.
 *
 * On the machine and not on the profile, the way the card's view is
 * (`cardView.ts`): it answers "what do I want to look at while I write", and
 * turning it off takes nothing out of the text - the accents are drawn over
 * the letters, never written into them.
 */

const STORAGE_KEY = 'kilna.versions.accents'

function stored(): boolean {
  try {
    return localStorage.getItem(STORAGE_KEY) === 'on'
  } catch {
    // A private window, or storage the machine refuses: off, and the text
    // reads the same.
    return false
  }
}

/** The preference, and a way to change it. Off until it is turned on. */
export function useShowStresses(): [boolean, (on: boolean) => void] {
  const [on, setOn] = useState(stored)
  const set = useCallback((next: boolean) => {
    setOn(next)
    try {
      if (next) localStorage.setItem(STORAGE_KEY, 'on')
      else localStorage.removeItem(STORAGE_KEY)
    } catch {
      // Not remembered, but still shown: the choice holds for this sitting.
    }
  }, [])
  return [on, set]
}
