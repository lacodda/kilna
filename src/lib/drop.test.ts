import { describe, expect, it } from 'vitest'
import { landsOn, picturesAmong } from '@/lib/drop'

const BOX = { left: 100, top: 50, right: 300, bottom: 250 }

describe('landsOn', () => {
  it('finds the box under the pointer, edges included', () => {
    expect(landsOn(BOX, { x: 150, y: 100 }, 1)).toBe(true)
    expect(landsOn(BOX, { x: 100, y: 250 }, 1)).toBe(true)
    expect(landsOn(BOX, { x: 99, y: 100 }, 1)).toBe(false)
    expect(landsOn(BOX, { x: 150, y: 251 }, 1)).toBe(false)
  })

  it('reads the pointer in physical pixels against a box in CSS ones', () => {
    // At 150%, physical (420, 300) is CSS (280, 200): on the box.
    expect(landsOn(BOX, { x: 420, y: 300 }, 1.5)).toBe(true)
    // Read as CSS pixels it would have missed.
    expect(landsOn(BOX, { x: 420, y: 300 }, 1)).toBe(false)
  })

  it('lands on nothing that is not on screen', () => {
    expect(landsOn(null, { x: 0, y: 0 }, 1)).toBe(false)
    expect(landsOn(undefined, { x: 0, y: 0 }, 1)).toBe(false)
  })
})

describe('picturesAmong', () => {
  it('takes the pictures and counts what it left', () => {
    expect(picturesAmong(['C:\\refs\\a.png', 'C:\\refs\\b.JPG', 'C:\\refs\\notes.txt'])).toEqual({
      pictures: ['C:\\refs\\a.png', 'C:\\refs\\b.JPG'],
      others: 1,
    })
    expect(picturesAmong([])).toEqual({ pictures: [], others: 0 })
  })
})
