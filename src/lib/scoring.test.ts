import { describe, expect, it } from 'vitest'
import type { Axis, Tier } from '@/lib/api'
import { markReaching, nextTier, rubricFor, tierFor, toNextTier, total } from '@/lib/scoring'

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

describe('nextTier', () => {
  it('picks the lowest tier still out of reach', () => {
    expect(nextTier(TIERS, 60)?.key).toBe('clip')
    expect(nextTier(TIERS, 10)?.key).toBe('audio')
  })

  it('has nothing above the top tier', () => {
    expect(nextTier(TIERS, 80)).toBeUndefined()
  })

  it('does not offer the tier the score is exactly on', () => {
    // Standing on the boundary means you are in that tier, not below it.
    expect(nextTier(TIERS, 50)?.key).toBe('clip')
  })
})

describe('toNextTier', () => {
  it('names the gap in points of the total', () => {
    // Four across the board is forty; the audio tier starts at fifty.
    const values = { hook: 4, lyrics: 4, emotion: 4 }
    const to = toNextTier(AXES, values, TIERS, total(AXES, values))

    expect(to?.tier.key).toBe('audio')
    expect(to?.gap).toBeCloseTo(10)
  })

  it('points at the heaviest axis when every axis stands equal', () => {
    // The advice is only worth anything if it is the cheapest route: with all
    // three axes on the same mark, one more mark on `hook` (weight 2) moves
    // the total further than one on `emotion` (weight 1).
    const values = { hook: 4, lyrics: 4, emotion: 4 }
    const to = toNextTier(AXES, values, TIERS, total(AXES, values))

    expect(to?.cheapest?.axis.key).toBe('hook')
  })

  it('prefers the light axis with room over the heavy one at its ceiling', () => {
    // The heavy axis cannot move at all - it is already at ten - so the answer
    // has to be the axis that can. A version that only compared weights would
    // keep recommending `hook` here and be useless.
    const values = { hook: 10, lyrics: 4, emotion: 4 }
    const to = toNextTier(AXES, values, TIERS, total(AXES, values))

    expect(to?.cheapest?.axis.key).not.toBe('hook')
    expect(to?.cheapest?.marks).toBeGreaterThan(0)
  })

  it('counts marks rather than weight when the scales differ', () => {
    // The trap: on axes that share a scale, the heaviest axis is always also
    // the cheapest, so "pick the heaviest" passes every test written with one
    // scale - and is wrong. A heavy axis on a fine scale moves the total very
    // little per mark: here `heavy` (weight 2, scale 100) needs fifteen marks
    // to reach fifty, while `coarse` (weight 1, scale 5) needs two.
    const scales: Axis[] = [
      { key: 'heavy', label: 'heavy', weight: 2, scale: 100 },
      { key: 'coarse', label: 'coarse', weight: 1, scale: 5 },
    ]
    const values = { heavy: 40, coarse: 2 }
    const to = toNextTier(scales, values, TIERS, total(scales, values))

    expect(to?.cheapest?.axis.key).toBe('coarse')
    expect(to?.cheapest?.marks).toBe(2)
  })

  it('says the marks it would actually take, and they do take it there', () => {
    const values = { hook: 4, lyrics: 4, emotion: 4 }
    const score = total(AXES, values)
    const to = toNextTier(AXES, values, TIERS, score)!
    const { axis, marks } = to.cheapest!

    // The claim is checkable: apply it and the total reaches the tier, and one
    // mark fewer does not. This is the test that a rounded-off or off-by-one
    // answer fails.
    const raised = total(AXES, { ...values, [axis.key]: values[axis.key as keyof typeof values]! + marks })
    const short = total(AXES, { ...values, [axis.key]: values[axis.key as keyof typeof values]! + marks - 1 })

    expect(raised).toBeGreaterThanOrEqual(to.tier.min)
    expect(short).toBeLessThan(to.tier.min)
  })

  it('offers no advice when no single axis can close the gap', () => {
    // Five across the board is fifty, and the clip tier is seventy-five: even
    // hook taken from five to ten only reaches 72.2. Better to say nothing
    // than to point at an axis that will not deliver.
    const values = { hook: 5, lyrics: 5, emotion: 5 }
    const to = toNextTier(AXES, values, TIERS, total(AXES, values))

    expect(to?.tier.key).toBe('clip')
    expect(to?.cheapest).toBeUndefined()
  })

  it('ignores an axis nobody has judged yet', () => {
    // An unjudged axis is outside the average, so raising it changes the
    // denominator too - a direction this arithmetic does not predict.
    const values = { hook: 4, lyrics: 4 }
    const to = toNextTier(AXES, values, TIERS, total(AXES, values))

    expect(to?.cheapest?.axis.key).not.toBe('emotion')
  })

  it('has nothing to say in the top tier', () => {
    const values = { hook: 10, lyrics: 10, emotion: 10 }
    expect(toNextTier(AXES, values, TIERS, total(AXES, values))).toBeUndefined()
  })
})

describe('markReaching', () => {
  it('is the lowest mark on the axis from which the total reaches the tier', () => {
    const values = { hook: 4, lyrics: 4, emotion: 4 }
    const mark = markReaching(AXES, values, AXES[0]!, 50)!

    // Checkable the same way: that mark gets there and the one below it does not.
    expect(total(AXES, { ...values, hook: mark })).toBeGreaterThanOrEqual(50)
    expect(total(AXES, { ...values, hook: mark - 1 })).toBeLessThan(50)
  })

  it('draws no notch when even full marks fall short', () => {
    // From five across, hook at ten reaches 72.2 - short of the clip tier.
    const values = { hook: 5, lyrics: 5, emotion: 5 }
    expect(markReaching(AXES, values, AXES[0]!, 75)).toBeUndefined()
  })

  it('draws no notch on a weightless axis, which cannot move the total', () => {
    const dead = axis('dead', 0)
    const withDead = [...AXES, dead]
    expect(markReaching(withDead, { hook: 4, lyrics: 4, emotion: 4, dead: 1 }, dead, 50)).toBeUndefined()
  })
})

describe('rubricFor', () => {
  const rubric = axis('hook', 2)
  rubric.rubric = [
    { at: 3, label: 'audible, but it does not catch' },
    { at: 7, label: 'the chorus sticks on the first listen' },
    { at: 10, label: 'still there the next day' },
  ]

  it('reads the nearest named mark below an unnamed one', () => {
    expect(rubricFor(rubric, 6)?.at).toBe(3)
  })

  it('reads a named mark as itself', () => {
    expect(rubricFor(rubric, 7)?.at).toBe(7)
  })

  it('has nothing to say below the first landmark', () => {
    expect(rubricFor(rubric, 1)).toBeUndefined()
  })

  it('has nothing to say for an axis with no rubric', () => {
    expect(rubricFor(AXES[1]!, 8)).toBeUndefined()
  })

  it('does not depend on the order the landmarks are listed in', () => {
    const shuffled = axis('hook', 2)
    shuffled.rubric = [rubric.rubric![2]!, rubric.rubric![0]!, rubric.rubric![1]!]
    expect(rubricFor(shuffled, 6)?.at).toBe(3)
  })
})
