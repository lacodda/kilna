/**
 * How much of a day the month has room to show.
 *
 * The month takes the window's height and splits it between its weeks, so a
 * day is as tall as the window makes it - not as tall as what is booked on it.
 * Until v0.79 a busy day grew, the week it sat in grew with it, and the grid
 * ran past the bottom of the window. Now a day holds as many one-line chips as
 * fit, and folds the rest into a count.
 *
 * The numbers are the ones `DayCell` draws with: a chip is `h-5` and two are
 * `gap-0.5` apart, and the "+N more" line is a chip's height too, so the sum
 * is exact rather than estimated.
 */

/** A chip's height, in pixels. */
export const CHIP_HEIGHT = 20

/** The gap between two chips, in pixels. */
export const CHIP_GAP = 2

/**
 * How many lines a day is assumed to have before it has been measured - the
 * first frame, and every test, since jsdom lays nothing out. Two chips and the
 * count under them, which is what a day showed before it was measured at all.
 */
export const UNMEASURED_LINES = 3

/** How many chip lines fit in a day's chip area of `height` pixels. */
export function linesThatFit(height: number): number {
  if (!Number.isFinite(height) || height <= 0) return 0
  return Math.floor((height + CHIP_GAP) / (CHIP_HEIGHT + CHIP_GAP))
}

/** What a day draws: its first `shown` chips, then `more` folded into a count. */
interface FoldedDay {
  shown: number
  more: number
}

/**
 * Fold a day's `count` chips into the `lines` it has room for.
 *
 * The count takes a line of its own, so a day that cannot show everything
 * gives up one chip more than the overflow: five chips in four lines is three
 * chips and "+2 more", never four chips and a count cut off at the bottom.
 */
export function foldDay(count: number, lines: number): FoldedDay {
  if (count <= lines) return { shown: count, more: 0 }
  const shown = Math.max(lines - 1, 0)
  return { shown, more: count - shown }
}
