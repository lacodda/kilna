import type { Axis, AxisMark, Tier } from '@/lib/api'

// The backend is the authority on what a score is worth — it recomputes on
// save, so two clients cannot disagree. This mirror exists only to show the
// total moving while the card is being filled in.

export function total(axes: Axis[], values: Record<string, number>): number {
  let weighted = 0
  let weightSum = 0

  for (const axis of axes) {
    const value = values[axis.key]
    if (value === undefined || axis.scale <= 0) continue
    weighted += (value / axis.scale) * axis.weight
    weightSum += axis.weight
  }

  // An empty card is zero rather than a division by zero.
  return weightSum === 0 ? 0 : (weighted / weightSum) * 100
}

export function tierFor(tiers: Tier[], score: number): Tier | undefined {
  return tiers
    .filter((tier) => score >= tier.min)
    .reduce<Tier | undefined>((best, tier) => (best === undefined || tier.min > best.min ? tier : best), undefined)
}

/** The tier above `score`, or `undefined` when it is already in the top one. */
export function nextTier(tiers: Tier[], score: number): Tier | undefined {
  return tiers
    .filter((tier) => tier.min > score)
    .reduce<Tier | undefined>((best, tier) => (best === undefined || tier.min < best.min ? tier : best), undefined)
}

/**
 * What the next tier would cost, and where it is cheapest to pay.
 *
 * The total is a weighted average, so a point is not a point: raising a heavy
 * axis by one moves the total further than raising a light one, and an axis
 * already at its ceiling cannot move it at all. `cheapest` is the axis where
 * the fewest marks close the gap - which is the advice a person actually wants
 * when they are looking at a number that is nearly enough.
 *
 * Unjudged axes are left out of the arithmetic entirely, the same way `total`
 * leaves them out: an empty axis has no weight in the average yet, so filling
 * one in moves the total in a direction this cannot predict.
 */
export interface ToNextTier {
  tier: Tier
  /** Points of total still missing. */
  gap: number
  /** The axis that closes it for the fewest marks, if any axis can. */
  cheapest?: { axis: Axis; marks: number }
}

export function toNextTier(
  axes: Axis[],
  values: Record<string, number>,
  tiers: Tier[],
  score: number,
): ToNextTier | undefined {
  const tier = nextTier(tiers, score)
  if (tier === undefined) return undefined

  const gap = tier.min - score

  let cheapest: ToNextTier['cheapest']
  for (const axis of axes) {
    const marks = marksToReach(axes, values, axis, tier.min)
    if (marks === undefined) continue
    if (cheapest === undefined || marks < cheapest.marks) cheapest = { axis, marks }
  }

  return { tier, gap, cheapest }
}

/**
 * How many more marks on `axis` would carry the total to `target`.
 *
 * `undefined` when this axis cannot get there: it is unjudged, weightless, or
 * already so high that its remaining marks are not enough. Whole marks,
 * because that is what the scale accepts - half a mark is not an answer.
 */
export function marksToReach(
  axes: Axis[],
  values: Record<string, number>,
  axis: Axis,
  target: number,
): number | undefined {
  const current = values[axis.key]
  if (current === undefined || axis.scale <= 0 || axis.weight <= 0) return undefined

  for (let mark = current + 1; mark <= axis.scale; mark += 1) {
    if (total(axes, { ...values, [axis.key]: mark }) >= target) return mark - current
  }
  return undefined
}

/**
 * The lowest mark on `axis` from which the total reaches `target`.
 *
 * This is the same question as `marksToReach` asked of the scale rather than
 * of the person - it is what the notch on an axis is drawn at. `undefined`
 * when no mark on this axis gets there, and the notch is then not drawn at
 * all: a line on a scale that cannot deliver what it promises is worse than no
 * line.
 */
export function markReaching(
  axes: Axis[],
  values: Record<string, number>,
  axis: Axis,
  target: number,
): number | undefined {
  if (axis.scale <= 0 || axis.weight <= 0) return undefined

  for (let mark = 1; mark <= axis.scale; mark += 1) {
    if (total(axes, { ...values, [axis.key]: mark }) >= target) return mark
  }
  return undefined
}

/**
 * The rubric sentence that covers `mark`: the nearest named mark at or below it.
 *
 * Landmarks are named, the marks between them are not, and a seven with
 * nothing of its own means what the profile said about the mark before it -
 * which is how a rubric is read on paper too.
 */
export function rubricFor(axis: Axis, mark: number): AxisMark | undefined {
  return (axis.rubric ?? [])
    .filter((entry) => entry.at <= mark)
    .reduce<AxisMark | undefined>((best, entry) => (best === undefined || entry.at > best.at ? entry : best), undefined)
}
