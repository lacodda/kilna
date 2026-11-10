import { describe, expect, it } from 'vitest'
import { toStep } from './step'

describe('toStep', () => {
  it('holds a number to its step from the bottom of its range', () => {
    expect(toStep(18.5, 1)).toBe(19)
    expect(toStep(18.4, 1)).toBe(18)
    expect(toStep(0.25, 0.5, -3)).toBe(0.5)
    expect(toStep(-1.4, 0.5, -3)).toBe(-1.5)
    expect(toStep(0.3, 0.1)).toBe(0.3)
  })

  it('keeps it inside its bounds', () => {
    expect(toStep(60, 1, 0, 51)).toBe(51)
    expect(toStep(-4, 1, 0, 51)).toBe(0)
  })

  it('leaves a number with no step as it was typed', () => {
    expect(toStep(18.5)).toBe(18.5)
    expect(toStep(18.5, 0)).toBe(18.5)
  })
})
