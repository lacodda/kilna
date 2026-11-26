/**
 * A small seeded generator: the same seed draws the same blots, on every
 * machine and in every build. A linear congruential one - what is drawn from
 * it is ink, not secrets.
 */
export type Rng = () => number

export function rng(seed: number): Rng {
  let state = seed >>> 0
  return () => {
    state = (state * 1664525 + 1013904223) >>> 0
    return state / 4294967296
  }
}
