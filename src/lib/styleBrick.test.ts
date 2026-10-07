import { describe, expect, it } from 'vitest'
import { colourFill, isColourForm } from '@/lib/styleBrick'

describe('colourFill', () => {
  it('paints one colour as itself and several as a gradient in their order', () => {
    expect(colourFill(['#FA8072'])).toBe('#FA8072')
    expect(colourFill(['#FA8072', '#F4A261', '#F5E3A3'])).toBe(
      'linear-gradient(135deg, #FA8072, #F4A261, #F5E3A3)',
    )
  })

  it('leaves out what is not a colour yet, and paints nothing without one', () => {
    expect(colourFill(['#FA8072', '#F4A'])).toBe('#FA8072')
    expect(colourFill(['', 'teal'])).toBeUndefined()
    expect(colourFill([])).toBeUndefined()
  })
})

describe('isColourForm', () => {
  it('is a ground and an accent, and nothing else', () => {
    expect(isColourForm('colour')).toBe(true)
    expect(isColourForm('accent')).toBe(true)
    expect(isColourForm('picture')).toBe(false)
    expect(isColourForm('lettering')).toBe(false)
    expect(isColourForm(undefined)).toBe(false)
  })
})
