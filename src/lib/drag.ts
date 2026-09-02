/**
 * The geometry of dragging a chip across the month.
 *
 * Everything here is a pure function of numbers, so the awkward parts — when a
 * press becomes a drag, when the month should turn — are tested rather than
 * discovered by dragging things around by hand. The React side holds the
 * listeners and the element; this holds the rules.
 */

/**
 * How far the pointer must travel before a press counts as a drag.
 *
 * A chip is also a button that opens the release, so every press is both a
 * candidate click and a candidate drag. Too small a threshold and a click with
 * an unsteady hand opens nothing; too large and the chip feels stuck. Eight
 * pixels is what the tab strip and the row menu already use.
 */
export const DRAG_THRESHOLD = 8

/** How wide the strips down either side of the grid are, in pixels. */
export const EDGE_WIDTH = 56

/**
 * How long the pointer must rest in an edge strip before the month turns, and
 * how long between turns after that.
 *
 * Slow enough to cross the strip on the way to a day near the edge without
 * setting anything off, fast enough that reaching next month is not a wait.
 */
export const EDGE_DELAY = 500

/** Whether the pointer has moved far enough from where it went down. */
export function isDrag(from: { x: number; y: number }, to: { x: number; y: number }): boolean {
  const dx = to.x - from.x
  const dy = to.y - from.y
  // Squared, to keep a square root out of a pointermove handler.
  return dx * dx + dy * dy >= DRAG_THRESHOLD * DRAG_THRESHOLD
}

/** Which way the month should turn while the pointer sits here, if at all. */
export type EdgeSide = -1 | 0 | 1

/**
 * The edge strip the pointer is in, as a month step.
 *
 * Only the horizontal axis: the grid is one month wide and does not scroll, so
 * up and down mean nothing here. A pointer outside the grid altogether counts
 * as being in the strip it left through — dragging past the window edge is how
 * someone asks for the next month emphatically, not a reason to stop.
 */
export function edgeOf(x: number, bounds: { left: number; right: number }): EdgeSide {
  if (x < bounds.left + EDGE_WIDTH) return -1
  if (x > bounds.right - EDGE_WIDTH) return 1
  return 0
}

/**
 * Where to draw the chip that follows the pointer.
 *
 * The offset is where inside the chip the pointer went down, so the chip does
 * not jump to align a corner with the cursor the moment it is picked up.
 */
export function ghostAt(
  pointer: { x: number; y: number },
  grab: { x: number; y: number },
): { left: number; top: number } {
  return { left: pointer.x - grab.x, top: pointer.y - grab.y }
}
