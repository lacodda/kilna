/*
 * The arithmetic behind LineChart, with no React in it.
 *
 * Split out like `track-segments` and `activity-weeks`: a product labelling
 * its own points, or checking its own domain sums, should not import a
 * component to get at the numbers.
 *
 * Two things live here and neither is obvious. Turning a series with holes in
 * it into drawable runs, and choosing the ticks on an axis - which is a
 * question about what reads as a round number, not about dividing a range into
 * equal parts.
 */

/** One reading. `value: null` is a measurement that was not taken - which is
 * not a reading of zero, and not a reason to move the ones after it. */
export interface Point {
  /** Position along the axis. Same units throughout: a timestamp, an index. */
  at: number
  value: number | null
}

export interface Bounds {
  /** The value the left edge stands for. */
  from: number
  /** The right edge. */
  to: number
  /** The bottom of the plot. */
  min: number
  /** The top. */
  max: number
}

/** A point placed in the box, in percentages: x from the left, y from the top
 * (SVG's own direction, so a larger value has a smaller y). */
export interface Placed {
  x: number
  y: number
  at: number
  value: number
}

/**
 * The bounds a series is drawn against.
 *
 * The y range does NOT start at zero by default, and that is a deliberate
 * departure from the bar's rule. A bar's length *is* the quantity, so its
 * baseline has to be zero or the length lies. A line's job is the shape of a
 * change, and a balance moving between 4,900 and 5,100 flattens into a
 * horizontal rule on a zero-based axis - the very thing the reader opened the
 * chart to see. What keeps it honest is that the axis is labelled: the ticks
 * say where the bottom is, so nobody reads the floor as nothing.
 *
 * A caller who wants zero states it, and for a quantity that is a count rather
 * than a level - requests, errors - they should.
 */
export function boundsOf(points: Point[], stated: Partial<Bounds> = {}): Bounds | null {
  const drawn = points.filter((point): point is Point & { value: number } => point.value !== null)
  if (drawn.length === 0) return null

  const values = drawn.map((point) => point.value)
  const positions = points.map((point) => point.at)

  const from = stated.from ?? Math.min(...positions)
  const to = stated.to ?? Math.max(...positions)

  let min = stated.min ?? Math.min(...values)
  let max = stated.max ?? Math.max(...values)

  /* A flat series has no range to scale against, and dividing by it would put
   * every point at the same y - or at NaN. Given a band around the value, the
   * line sits in the middle of the plot and reads as what it is: unchanging. */
  if (!(max > min)) {
    const pad = Math.abs(max) > 0 ? Math.abs(max) * 0.1 : 1
    min = max - pad
    max = max + pad
  }

  return { from, to, min, max }
}

/**
 * The series as runs of consecutive readings.
 *
 * A hole breaks the line rather than being drawn through or closed up, and the
 * three options are worth naming because two of them lie:
 *
 *   **Interpolating across** invents a reading nobody took - the one thing a
 *   chart of measurements must never do.
 *
 *   **Dropping the point** keeps the line whole and moves every later reading
 *   to the left, so the axis stops matching the data: a chart that says the
 *   value was 40 in March when it was 40 in April.
 *
 *   **Breaking the line** leaves the gap visible, keeps every other point
 *   where it belongs, and invents nothing. So that is what happens here.
 *
 * A run of one point is kept: it has no line, but it has a dot, and dropping
 * it would hide a reading that exists.
 */
export function runs(points: Point[], bounds: Bounds): Placed[][] {
  const span = bounds.to - bounds.from
  const height = bounds.max - bounds.min
  if (!(span > 0) || !(height > 0)) return []

  const out: Placed[][] = []
  let run: Placed[] = []

  for (const point of points) {
    if (point.value === null) {
      if (run.length > 0) out.push(run)
      run = []
      continue
    }

    const clamped = Math.min(Math.max(point.value, bounds.min), bounds.max)
    run.push({
      x: ((point.at - bounds.from) / span) * 100,
      // SVG's y grows downward, so the largest value sits at the top.
      y: ((bounds.max - clamped) / height) * 100,
      at: point.at,
      value: point.value,
    })
  }

  if (run.length > 0) out.push(run)
  return out
}

/** A run as an SVG path, in the same percentages.
 *
 * Straight segments rather than a curve: a spline through measured points
 * overshoots between them, inventing highs and lows that were never recorded -
 * the same lie as interpolating across a gap, drawn more prettily. */
export function pathOf(run: Placed[]): string {
  return run
    .map((point, index) => `${index === 0 ? 'M' : 'L'}${point.x.toFixed(2)} ${point.y.toFixed(2)}`)
    .join(' ')
}

/**
 * Ticks for the value axis, on round numbers.
 *
 * Not the range cut into equal parts: 4,900 to 5,100 in four gives 4,950 and
 * 5,050, which nobody reads as a landmark. A tick's job is to be recognised
 * instantly, so the step is the nearest 1, 2, 5 or 10 above what the range
 * needs, and the ticks are the multiples of it inside the range.
 *
 * Returns nothing when a step would not fit - a range too narrow for a round
 * number is better with no ticks than with invented ones.
 */
export function ticksFor(bounds: Bounds, wanted = 4): number[] {
  const height = bounds.max - bounds.min
  if (!(height > 0) || wanted < 1) return []

  const rough = height / wanted
  const magnitude = 10 ** Math.floor(Math.log10(rough))
  const step = [1, 2, 5, 10].map((n) => n * magnitude).find((candidate) => candidate >= rough)
  if (step === undefined) return []

  const ticks: number[] = []
  for (let tick = Math.ceil(bounds.min / step) * step; tick <= bounds.max; tick += step) {
    // Floating point leaves 0.30000000000000004 where 0.3 was meant; the step
    // is what decides how many decimals are real.
    const decimals = Math.max(0, -Math.floor(Math.log10(step)))
    ticks.push(Number(tick.toFixed(decimals)))
  }

  return ticks
}

/** Where a value sits in the plot, as a percentage from the top - for a tick's
 * rule, or a threshold drawn across the line. */
export function yOf(value: number, bounds: Bounds): number | null {
  const height = bounds.max - bounds.min
  if (!(height > 0)) return null
  if (value < bounds.min || value > bounds.max) return null
  return ((bounds.max - value) / height) * 100
}
