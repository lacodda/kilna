import { useEffect, useState } from 'react'

/**
 * A value that trails its source, so a round trip is not made per keystroke.
 *
 * Typing is faster than SQLite is slow, but not by much once a workspace holds
 * a few hundred bodies — and both search boxes in the app ask the backend. The
 * delay is the caller's, because a palette that answers as you type and a
 * catalogue that narrows a whole table can reasonably differ.
 */
export function useDebounced<T>(value: T, delay: number): T {
  const [settled, setSettled] = useState(value)

  useEffect(() => {
    const timer = setTimeout(() => setSettled(value), delay)
    return () => clearTimeout(timer)
  }, [value, delay])

  return settled
}
