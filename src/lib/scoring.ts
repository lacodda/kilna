import type { Axis, Tier } from '@/lib/api'

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
