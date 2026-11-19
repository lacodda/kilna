import { useEffect, useState } from 'react'

/** How many rows are drawn at first, and how many more each time. */
export const WINDOW_STEP = 150

/**
 * How many of a long list's rows to draw (v0.163): a first step, and one
 * more each time a sentinel placed after the last drawn row comes near the
 * edge of what scrolls. A catalogue of five hundred works opens with the
 * first hundred and fifty and fills in as it is read, rather than laying out
 * every row before anything can be seen.
 *
 * The list only grows - what was drawn stays, so a scroll back up finds its
 * rows where it left them. Where the page cannot observe (a test's DOM, an
 * old engine) every row is drawn at once, as before.
 */
export function useGrowingWindow(total: number): {
  shown: number
  sentinel: (node: Element | null) => void
} {
  const observable = typeof IntersectionObserver !== 'undefined'
  const [shown, setShown] = useState(observable ? WINDOW_STEP : Number.POSITIVE_INFINITY)
  const [node, setNode] = useState<Element | null>(null)

  useEffect(() => {
    if (node === null || !observable) return
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          setShown((before) => before + WINDOW_STEP)
        }
      },
      // A screen ahead: the next rows are laid out before they are reached.
      { rootMargin: '800px 0px' },
    )
    observer.observe(node)
    return () => observer.disconnect()
  }, [node, observable, shown])

  return { shown: Math.min(shown, total), sentinel: setNode }
}
