import { useEffect, useState, type RefObject } from 'react'

/**
 * Which ends of a strip that scrolls sideways have more beyond them.
 *
 * A strip that hides its scrollbar - the card's tabs, twelve of them in a
 * window narrow enough to hold eight - has to say another way that it goes
 * on, or the tabs past the edge do not exist for anyone who did not already
 * know them. The strip draws a fade and a chevron at each end that has more.
 */
export interface Edges {
  /** There is more before the first thing in view. */
  start: boolean
  /** There is more after the last. */
  end: boolean
}

const NO_EDGES: Edges = { start: false, end: false }

/**
 * The ends of a strip, from its measurements.
 *
 * Within a pixel counts as there: a zoomed window scrolls by fractions, and
 * a strip scrolled to 0.4px from its end would otherwise draw a chevron that
 * leads nowhere.
 */
export function edgesOf({
  scrollLeft,
  scrollWidth,
  clientWidth,
}: {
  scrollLeft: number
  scrollWidth: number
  clientWidth: number
}): Edges {
  return {
    start: scrollLeft > 1,
    end: scrollLeft + clientWidth < scrollWidth - 1,
  }
}

/**
 * The ends of `strip` that have more beyond them, kept up to date.
 *
 * Measured when the strip scrolls and when either it or `content` - the row
 * inside it - changes size. The row matters on its own: a count arriving
 * beside a tab or the language changing widens what the strip holds without
 * the strip itself changing size, and neither fires a scroll.
 */
export function useEdges(
  strip: RefObject<HTMLElement | null>,
  content: RefObject<HTMLElement | null>,
): Edges {
  const [edges, setEdges] = useState<Edges>(NO_EDGES)

  useEffect(() => {
    const element = strip.current
    if (element === null) return
    const measure = () => {
      const next = edgesOf(element)
      // The same answer keeps the same object, so a scroll does not render
      // the strip again for every pixel it moves.
      setEdges((previous) =>
        previous.start === next.start && previous.end === next.end ? previous : next,
      )
    }
    measure()
    element.addEventListener('scroll', measure, { passive: true })
    const observer = new ResizeObserver(measure)
    observer.observe(element)
    if (content.current !== null) observer.observe(content.current)
    return () => {
      element.removeEventListener('scroll', measure)
      observer.disconnect()
    }
  }, [strip, content])

  return edges
}
