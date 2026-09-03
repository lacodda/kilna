import { useEffect, useRef, useState } from 'react'
import { useNavigate } from 'react-router'
import { opensChord, readChord, readStroke, typing, type Stroke } from '@/lib/keys'

/** How long a `g` waits for its second key before giving up, in milliseconds.
 *
 * Long enough not to hurry anyone, short enough that a forgotten `g` cannot
 * turn a later keystroke into a jump. Vim uses roughly a second for the same
 * reason; this is that, rounded to something a person would say out loud. */
const CHORD_WINDOW = 1500

/** Read the fields this cares about off a real keyboard event. */
function strokeOf(event: KeyboardEvent): Stroke {
  return {
    key: event.key,
    ctrlKey: event.ctrlKey,
    metaKey: event.metaKey,
    altKey: event.altKey,
    shiftKey: event.shiftKey,
    typing: typing(event.target),
  }
}

/**
 * The shell's keyboard.
 *
 * One listener on the window, because the alternative is a listener per screen
 * and a rule about which of them wins. What it does with a keystroke is
 * `keys.ts`'s decision; what is here is the part that cannot be a pure
 * function: the pending `g`, the timer that drops it, and the navigation.
 *
 * Returns whether the shortcut sheet is open, and the setter for it, so that
 * the same `?` works from anywhere and the sheet itself stays a component.
 */
export function useKeys(): { helpOpen: boolean; setHelpOpen: (open: boolean) => void } {
  const navigate = useNavigate()
  const [helpOpen, setHelpOpen] = useState(false)

  // A ref rather than state: nothing renders differently while a chord is
  // pending, and re-rendering the whole shell on every `g` would be a strange
  // price for a flag only the handler reads.
  const chording = useRef<number | null>(null)

  useEffect(() => {
    const drop = () => {
      if (chording.current !== null) {
        window.clearTimeout(chording.current)
        chording.current = null
      }
    }

    const onKey = (event: KeyboardEvent) => {
      const stroke = strokeOf(event)

      // A chord in flight takes the next key, whatever it is: `g` then a
      // letter that goes nowhere means the chord was a mistake, and the key
      // should not then act on its own.
      if (chording.current !== null) {
        drop()
        const intent = readChord(stroke)
        if (intent?.kind === 'go') {
          event.preventDefault()
          navigate(intent.where)
        }
        return
      }

      if (opensChord(stroke)) {
        chording.current = window.setTimeout(drop, CHORD_WINDOW)
        return
      }

      const intent = readStroke(stroke)
      if (intent === null) return

      event.preventDefault()
      if (intent.kind === 'help') setHelpOpen(true)
      else if (intent.kind === 'history') navigate(intent.delta)
    }

    window.addEventListener('keydown', onKey)
    return () => {
      window.removeEventListener('keydown', onKey)
      drop()
    }
  }, [navigate])

  return { helpOpen, setHelpOpen }
}
