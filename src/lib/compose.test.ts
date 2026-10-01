import { describe, expect, it } from 'vitest'
import { markPlacement, typeOf } from '@/lib/compose'

/*
 * Where an exported cover gets its mark: the box the backend gives - the
 * same one the scheme draws - in the picture's own pixels, as the largest
 * square inside it, pushed into the box's corner.
 */

describe('the mark on an exported cover', () => {
  it('sits in the lower right corner of a wide picture, square', () => {
    // The box of a 16:9 cover's lower right corner, as shares.
    const box = { x: 0.8538, y: 0.74, w: 0.1238, h: 0.22 }
    const at = markPlacement(box, 1600, 900)
    expect(at.side).toBeCloseTo(198, 0)
    expect(at.x + at.side).toBeCloseTo(1600 * (box.x + box.w), 0)
    expect(at.y + at.side).toBeCloseTo(900 * (box.y + box.h), 0)
  })

  it('keeps to the corner of its box when the picture is of another shape', () => {
    // A square picture given a 16:9 box: the box is wider than tall there,
    // and the square goes to the box's right edge, not its middle.
    const box = { x: 0.8538, y: 0.74, w: 0.1238, h: 0.22 }
    const at = markPlacement(box, 1000, 1000)
    expect(at.side).toBeCloseTo(123.8, 0)
    expect(at.x + at.side).toBeCloseTo(1000 * (box.x + box.w), 0)
    expect(at.y + at.side).toBeCloseTo(1000 * (box.y + box.h), 0)
  })

  it('sits in the upper left corner when the box is there', () => {
    const at = markPlacement({ x: 0.0225, y: 0.04, w: 0.1238, h: 0.22 }, 1600, 900)
    expect(at.x).toBeCloseTo(36, 0)
    expect(at.y).toBeCloseTo(36, 0)
  })

  it('reads a file by its ending, and a PNG when the ending says nothing', () => {
    expect(typeOf('C:/media/mark.svg')).toBe('image/svg+xml')
    expect(typeOf('cover.JPG')).toBe('image/jpeg')
    expect(typeOf('no-ending')).toBe('image/png')
  })
})
