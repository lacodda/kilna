import { describe, expect, it } from 'vitest'
import type { ProfileConfig, StyleBrick, StyleType } from '@/lib/api/types'
import { byOffer, colourFill, isColourForm, isOffered, withFamilyOff } from '@/lib/styleBrick'

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

describe('what the dictionary offers (v0.132)', () => {
  const types = [
    {
      key: 'image-style',
      label: 'Image style',
      families: [
        { key: 'classic', label: 'Classic' },
        { key: 'street', label: 'Street', off: true },
      ],
    },
  ] as unknown as StyleType[]
  const brick = (id: string, family: string | null, extra: Partial<StyleBrick> = {}) =>
    ({ id, type_key: 'image-style', family, status: 'ready', ...extra }) as StyleBrick

  it('offers a ready brick of a set that is on, and the owner’s own always', () => {
    expect(isOffered(types, brick('a', 'classic'))).toBe(true)
    expect(isOffered(types, brick('b', 'street'))).toBe(false)
    expect(isOffered(types, brick('c', null))).toBe(true)
    expect(isOffered(types, brick('d', 'classic', { status: 'dropped' }))).toBe(false)
  })

  it('lifts the house styles, then the starred', () => {
    const rows = [
      brick('plain', null),
      brick('star', null, { favorite: true }),
      brick('house', null),
    ]
    expect(rows.sort(byOffer(new Set(['house']))).map((one) => one.id)).toEqual([
      'house',
      'star',
      'plain',
    ])
  })

  it('switches a set off and on again, saying nothing of a set that is on', () => {
    const config = { style_types: types } as unknown as ProfileConfig
    const on = withFamilyOff(config, 'image-style', 'street', false)
    expect(on.style_types[0]!.families![1]).toEqual({ key: 'street', label: 'Street' })
    const off = withFamilyOff(on, 'image-style', 'classic', true)
    expect(off.style_types[0]!.families![0]).toEqual({
      key: 'classic',
      label: 'Classic',
      off: true,
    })
  })
})
