import { describe, expect, it } from 'vitest'
import type { Axis, Tier } from '@/lib/api'
import { tierFor, total } from '@/lib/scoring'

const axis = (key: string, weight: number, scale = 10): Axis => ({ key, label: key, weight, scale })

const AXES: Axis[] = [axis('hook', 2), axis('lyrics', 1.5), axis('emotion', 1)]

const TIERS: Tier[] = [
  { key: 'hold', label: 'Hold', min: 0 },
  { key: 'audio', label: 'Audio', min: 50 },
  { key: 'clip', label: 'Clip', min: 75 },
]

describe('total', () => {
  it('is the weighted average as a percentage of the scale', () => {
    // Every axis at eight out of ten is eighty, whatever the weights are.
    expect(total(AXES, { hook: 8, lyrics: 8, emotion: 8 })).toBeCloseTo(80)
  })

  it('weighs a heavier axis more', () => {
    // The same two marks swapped between a heavy axis and a light one: the
    // total is higher when the good mark sits on the heavier axis. Comparing
    // one high axis against two, as a first draft of this test did, measures
    // the combined weight instead and says nothing about weighting.
    const two = [axis('heavy', 3), axis('light', 1)]
    expect(total(two, { heavy: 10, light: 2 })).toBeGreaterThan(
      total(two, { heavy: 2, light: 10 }),
    )
  })

  // The rule the docs promise: an axis you did not judge is skipped, not
  // counted as zero. Getting this wrong punishes a half-filled card, which is
  // exactly when someone is still deciding.
  it('skips an unjudged axis instead of scoring it zero', () => {
    const partial = total(AXES, { hook: 8 })
    const withZeroes = total(AXES, { hook: 8, lyrics: 0, emotion: 0 })

    expect(partial).toBeCloseTo(80)
    expect(withZeroes).toBeLessThan(partial)
  })

  it('is zero for an empty card rather than dividing by nothing', () => {
    expect(total(AXES, {})).toBe(0)
    expect(Number.isFinite(total(AXES, {}))).toBe(true)
  })

  it('ignores an axis whose scale is zero rather than dividing by it', () => {
    const broken = [...AXES, axis('broken', 1, 0)]
    expect(Number.isFinite(total(broken, { hook: 8, broken: 5 }))).toBe(true)
  })

  it('reads a value on a different scale as its own fraction', () => {
    // Five out of five is full marks; five out of ten is half.
    expect(total([axis('a', 1, 5)], { a: 5 })).toBeCloseTo(100)
    expect(total([axis('a', 1, 10)], { a: 5 })).toBeCloseTo(50)
  })
})

describe('tierFor', () => {
  it('picks the highest tier the score reaches', () => {
    expect(tierFor(TIERS, 80)?.key).toBe('clip')
    expect(tierFor(TIERS, 60)?.key).toBe('audio')
    expect(tierFor(TIERS, 10)?.key).toBe('hold')
  })

  it('counts a score exactly on the boundary as reaching it', () => {
    expect(tierFor(TIERS, 75)?.key).toBe('clip')
    expect(tierFor(TIERS, 74.9)?.key).toBe('audio')
  })

  it('has nothing to say when no tier starts low enough', () => {
    expect(tierFor([{ key: 'clip', label: 'Clip', min: 75 }], 10)).toBeUndefined()
  })

  it('does not depend on the order the tiers are listed in', () => {
    const shuffled = [TIERS[2]!, TIERS[0]!, TIERS[1]!]
    expect(tierFor(shuffled, 60)?.key).toBe('audio')
  })
})
