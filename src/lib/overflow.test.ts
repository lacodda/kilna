import { describe, expect, it } from 'vitest'
import { edgesOf } from '@/lib/overflow'

describe('edgesOf', () => {
  it('says nothing of a strip that holds everything', () => {
    expect(edgesOf({ scrollLeft: 0, scrollWidth: 400, clientWidth: 400 })).toEqual({
      start: false,
      end: false,
    })
  })

  it('marks the end with more beyond it, and only that end', () => {
    expect(edgesOf({ scrollLeft: 0, scrollWidth: 900, clientWidth: 400 })).toEqual({
      start: false,
      end: true,
    })
    expect(edgesOf({ scrollLeft: 500, scrollWidth: 900, clientWidth: 400 })).toEqual({
      start: true,
      end: false,
    })
    expect(edgesOf({ scrollLeft: 200, scrollWidth: 900, clientWidth: 400 })).toEqual({
      start: true,
      end: true,
    })
  })

  it('takes a strip within a pixel of its end as at the end', () => {
    // A zoomed window scrolls by fractions of a pixel.
    expect(edgesOf({ scrollLeft: 499.6, scrollWidth: 900, clientWidth: 400 }).end).toBe(false)
    expect(edgesOf({ scrollLeft: 0.6, scrollWidth: 900, clientWidth: 400 }).start).toBe(false)
  })
})
