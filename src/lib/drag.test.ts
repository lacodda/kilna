import { describe, expect, it } from 'vitest'
import { DRAG_THRESHOLD, EDGE_WIDTH, edgeOf, ghostAt, isDrag } from './drag'

describe('isDrag', () => {
  const origin = { x: 100, y: 100 }

  it('does not start on a press that has not moved', () => {
    expect(isDrag(origin, origin)).toBe(false)
  })

  // The chip is a button as well: a click with an unsteady hand must still
  // open the release rather than pick it up and drop it where it was.
  it('does not start on the small wobble of a click', () => {
    expect(isDrag(origin, { x: 103, y: 102 })).toBe(false)
  })

  it('starts once the pointer has travelled the threshold', () => {
    expect(isDrag(origin, { x: 100 + DRAG_THRESHOLD, y: 100 })).toBe(true)
    expect(isDrag(origin, { x: 100, y: 100 - DRAG_THRESHOLD })).toBe(true)
  })

  // Distance, not axes: eight across and eight up is further than either.
  it('measures the distance rather than each axis', () => {
    const diagonal = Math.ceil(DRAG_THRESHOLD / Math.SQRT2)
    expect(isDrag(origin, { x: 100 + diagonal, y: 100 + diagonal })).toBe(true)
    expect(isDrag(origin, { x: 100 + diagonal - 2, y: 100 })).toBe(false)
  })

  it('does not care which way the pointer went', () => {
    const far = DRAG_THRESHOLD + 5
    const ways: { dx: number; dy: number }[] = [
      { dx: far, dy: 0 },
      { dx: -far, dy: 0 },
      { dx: 0, dy: far },
      { dx: 0, dy: -far },
    ]
    for (const { dx, dy } of ways) {
      expect(isDrag(origin, { x: 100 + dx, y: 100 + dy })).toBe(true)
    }
  })
})

describe('edgeOf', () => {
  const grid = { left: 200, right: 1000 }

  it('is quiet in the middle', () => {
    expect(edgeOf(600, grid)).toBe(0)
  })

  it('turns back near the left and forward near the right', () => {
    expect(edgeOf(grid.left + 10, grid)).toBe(-1)
    expect(edgeOf(grid.right - 10, grid)).toBe(1)
  })

  it('is quiet just inside either strip', () => {
    expect(edgeOf(grid.left + EDGE_WIDTH + 1, grid)).toBe(0)
    expect(edgeOf(grid.right - EDGE_WIDTH - 1, grid)).toBe(0)
  })

  // Dragging past the window edge is someone asking for the next month
  // emphatically, not a reason to stop turning.
  it('keeps turning when the pointer leaves the grid entirely', () => {
    expect(edgeOf(grid.left - 300, grid)).toBe(-1)
    expect(edgeOf(grid.right + 300, grid)).toBe(1)
  })

  // A grid narrower than two strips would otherwise report both sides at once
  // and the month would flicker; the left wins, deterministically.
  it('does not report both sides on a grid narrower than its strips', () => {
    const narrow = { left: 0, right: EDGE_WIDTH }
    expect(edgeOf(EDGE_WIDTH / 2, narrow)).toBe(-1)
  })
})

describe('ghostAt', () => {
  it('keeps the chip where it was grabbed under the pointer', () => {
    // Pressed 30px in and 8px down from the chip's corner: wherever the
    // pointer goes, that same point of the chip stays under it.
    expect(ghostAt({ x: 500, y: 400 }, { x: 30, y: 8 })).toEqual({ left: 470, top: 392 })
  })

  it('does not jump when the chip is grabbed at its corner', () => {
    expect(ghostAt({ x: 500, y: 400 }, { x: 0, y: 0 })).toEqual({ left: 500, top: 400 })
  })
})
