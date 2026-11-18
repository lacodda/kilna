import { describe, expect, it } from 'vitest'
import { clipped, forceLayout, ring, toneOf } from './graph'

// v0.143, FR-18 18.4: the canon drawn - the same cards land in the same
// places, inside the drawing, the related ones nearer each other.
describe('the graph of the canon', () => {
  it('puts the cards around one on a ring, the first at the top', () => {
    const [first, second] = ring(4, 10, { x: 50, y: 50 })
    expect(first!.x).toBeCloseTo(50)
    expect(first!.y).toBeCloseTo(40)
    expect(second!.x).toBeCloseTo(60)
    expect(second!.y).toBeCloseTo(50)
  })

  it('lays the canon out the same way every time, inside the drawing', () => {
    const ids = ['a', 'b', 'c', 'd', 'e']
    const links = [
      { from: 'a', to: 'b' },
      { from: 'b', to: 'c' },
    ]
    const once = forceLayout(ids, links, { width: 400, height: 300 })
    const again = forceLayout(ids, links, { width: 400, height: 300 })
    expect([...once.entries()]).toEqual([...again.entries()])
    for (const spot of once.values()) {
      expect(spot.x).toBeGreaterThanOrEqual(40)
      expect(spot.x).toBeLessThanOrEqual(360)
      expect(spot.y).toBeGreaterThanOrEqual(40)
      expect(spot.y).toBeLessThanOrEqual(260)
    }
    const apart = (one: string, other: string) =>
      Math.hypot(once.get(one)!.x - once.get(other)!.x, once.get(one)!.y - once.get(other)!.y)
    expect(apart('a', 'b')).toBeLessThan(apart('a', 'e'))
  })

  it('puts one card in the middle, and none nowhere', () => {
    expect(forceLayout(['a'], [], { width: 200, height: 100 }).get('a')).toEqual({ x: 100, y: 50 })
    expect(forceLayout([], [], { width: 200, height: 100 }).size).toBe(0)
  })

  it('cuts a long label and colours a kind by its place in the profile', () => {
    expect(clipped('Neighbour and first listener', 14)).toBe('Neighbour and…')
    expect(clipped('Sister')).toBe('Sister')
    expect(toneOf('location', ['character', 'location'])).toContain('info')
    expect(toneOf('nonesuch', ['character'])).toContain('fill-soft')
  })
})
