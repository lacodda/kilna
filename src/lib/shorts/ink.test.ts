import { describe, expect, it } from 'vitest'
import type { ShortStyle } from '@/lib/api/types'
import { artSrc, blotsOf, groundSvg, stampSvg, wearFilter, wearSeed } from '@/lib/shorts/ink'
import { rng } from '@/lib/shorts/rng'
import { isColour, pickerValue } from '@/lib/colour'
import { freeKey, worn } from '@/lib/shorts/style'

const STYLE: ShortStyle = {
  accent: '#FF2E63',
  second: '#D3EC3F',
  header: { on: true, title_upper: true, call: 'the whole song is on the channel', font: 'Arial' },
  sign: {
    on: true,
    art: '<svg id="own"/>',
    blink: '<svg id="shut"/>',
    size: 310,
    pulse: 0.24,
    sway: 5,
  },
  ground: {
    color: '#121114',
    pattern: 'blots',
    seed: 9001,
    motion: true,
    blots: [
      { x: 60, y: 340, size: 1250, color: 'accent', shape: 'splat' },
      { x: 1010, y: 110, size: 820, color: 'second', shape: 'drip' },
    ],
  },
  stamp: {
    on: true,
    shape: 'stamp',
    text: 'Harbour <Radio>',
    sub: 'music',
    art: '',
    color: '#D7262E',
    angle: -6,
    x: 610,
    wear: 0.6,
  },
  wave: { on: true, kind: 'solid', strip: true },
  karaoke: { on: true, font: 'Arial', size: 60 },
  effects: { zoom: 0.1, grain: 7, vignette: true, glitch: true },
  tail: { on: true, seconds: 1.2 },
  variants: [
    { key: 'day', label: 'Day', accent: '#FFD23F', stamp_angle: 5, wave: 'bars' },
    {
      key: 'night',
      sign_art: '<svg id="night"/>',
      ground: { pattern: 'dots', seed: 7, blots: [] },
    },
  ],
}

describe('the ink of a short', () => {
  it('draws the same numbers from the same seed', () => {
    const a = rng(451)
    const b = rng(451)
    const first = [a(), a(), a()]
    expect([b(), b(), b()]).toEqual(first)
    expect(first.every((n) => n >= 0 && n < 1)).toBe(true)
    expect(rng(452)()).not.toBe(first[0])
  })

  it('draws the same blots from the same style, each in its colour and where its heart was put', () => {
    const blots = blotsOf(STYLE)
    expect(blotsOf(STYLE)).toEqual(blots)
    expect(blots).toHaveLength(2)
    // A blot's heart is at (450, 400) of its 900 canvas: its corner stands
    // half its size left of the heart and four ninths of it above.
    expect(blots[0]?.x).toBe(60 - 625)
    expect(blots[0]?.y).toBeCloseTo(340 - (1250 * 400) / 900)
    expect(blots[0]?.svg).toContain('fill="#FF2E63"')
    expect(blots[1]?.svg).toContain('fill="#D3EC3F"')
    expect(blots[0]?.svg).toContain('filter="url(#goo)"')
    for (const blot of blots) {
      expect(blot.ax).toBeGreaterThanOrEqual(12)
      expect(blot.px).toBeGreaterThanOrEqual(6)
      expect(Math.abs(blot.kick)).toBeGreaterThanOrEqual(22)
    }
    // Another seed is other ink in the same places.
    const other = blotsOf({ ...STYLE, ground: { ...STYLE.ground, seed: 9002 } })
    expect(other[0]?.x).toBe(blots[0]?.x)
    expect(other[0]?.svg).not.toBe(blots[0]?.svg)
  })

  it('has no blots on a ground of another pattern, and draws the pattern instead', () => {
    const dotted: ShortStyle = { ...STYLE, ground: { ...STYLE.ground, pattern: 'dots' } }
    expect(blotsOf(dotted)).toEqual([])
    expect(groundSvg(dotted).match(/<circle/g)).toHaveLength(260)
    expect(groundSvg(STYLE).match(/<circle/g)).toHaveLength(40)
    const plain = groundSvg({ ...STYLE, ground: { ...STYLE.ground, pattern: 'plain' } })
    expect(plain).toContain('fill="#121114"')
    expect(plain).not.toContain('<circle')
    expect(groundSvg({ ...STYLE, ground: { ...STYLE.ground, pattern: 'grid' } })).toContain(
      'opacity="0.12"',
    )
  })

  it('letters the stamp with the name as text, worn, and none without a name', () => {
    const stamp = stampSvg(STYLE.stamp)
    expect(stamp?.width).toBe(650)
    expect(stamp?.svg).toContain('HARBOUR &lt;RADIO&gt;')
    expect(stamp?.svg).toContain('MUSIC')
    expect(stamp?.svg).toContain('filter="url(#wear)"')
    expect(stampSvg({ ...STYLE.stamp, wear: 0 })?.svg).not.toContain('url(#wear)')
    expect(stampSvg({ ...STYLE.stamp, shape: 'plate' })?.svg).toContain('mask="url(#cut)"')
    expect(stampSvg({ ...STYLE.stamp, text: '  ' })).toBeNull()
    expect(wearFilter(0, 1)).toBe('')
    expect(wearSeed('Harbour')).toBe(wearSeed('Harbour'))
    expect(wearSeed('Harbour')).not.toBe(wearSeed('Tide'))
  })

  it('reads a picture kept as text: an SVG as a data address, anything else as it is', () => {
    expect(artSrc('  ')).toBe('')
    expect(artSrc('<svg/>')).toBe('data:image/svg+xml;charset=utf-8,%3Csvg%2F%3E')
    expect(artSrc('data:image/png;base64,AAAA')).toBe('data:image/png;base64,AAAA')
  })
})

describe('a style as a variant wears it', () => {
  it('replaces what the variant names and leaves the rest', () => {
    const day = worn(STYLE, STYLE.variants[0])
    expect(day.accent).toBe('#FFD23F')
    expect(day.second).toBe('#D3EC3F')
    expect(day.stamp.angle).toBe(5)
    expect(day.wave.kind).toBe('bars')
    expect(day.sign.blink).toBe('<svg id="shut"/>')
    expect(day.variants).toEqual([])

    const night = worn(STYLE, STYLE.variants[1])
    expect(night.sign.art).toBe('<svg id="night"/>')
    // Another sign does not blink with the style's eyes.
    expect(night.sign.blink).toBe('')
    expect(night.ground.pattern).toBe('dots')
    expect(night.ground.color).toBe('#121114')

    expect(worn(STYLE, undefined).accent).toBe('#FF2E63')
    // The style itself is not changed by being worn.
    expect(STYLE.stamp.angle).toBe(-6)
  })

  it('finds a key no variant has, and knows a colour when it sees one', () => {
    expect(freeKey(STYLE)).toBe('variant-3')
    expect(
      freeKey({ ...STYLE, variants: [{ key: 'variant-2' }, { key: 'variant-3' }, { key: 'x' }] }),
    ).toBe('variant-4')
    expect(isColour('#FF2E63')).toBe(true)
    expect(isColour('#F2E')).toBe(false)
    expect(isColour('red')).toBe(false)
    expect(pickerValue('#ff2e63')).toBe('#ff2e63')
    expect(pickerValue('re')).not.toBe('re')
  })
})
