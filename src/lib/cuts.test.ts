import { describe, expect, it } from 'vitest'
import type { Cut, Shot } from '@/lib/api'
import { bandsOf, blockerOf, canBeCut, lengthOf, orderMoving, totalLength, tracksOf } from '@/lib/cuts'

function cut(over: Partial<Cut> & Pick<Cut, 'starts_at' | 'ends_at'>): Cut {
  return {
    id: 'c1',
    profile_id: 'p',
    work_id: 'w',
    source_id: 's1',
    position: 1,
    label: null,
    created_at: '',
    updated_at: '',
    source_title: 'Harbour lights',
    source_duration: 200,
    ...over,
  }
}

function shot(over: Partial<Shot> = {}): Shot {
  return {
    cut_id: 'c1',
    position: 1,
    starts_at: 10,
    ends_at: 22,
    source_id: 's1',
    source_title: 'Harbour lights',
    path: '/media/harbour.mp4',
    ...over,
  }
}

describe('lengths', () => {
  it('adds every stretch up, because that is how long the short runs', () => {
    const splice = [cut({ starts_at: 10, ends_at: 22 }), cut({ id: 'c2', starts_at: 90, ends_at: 98 })]
    expect(lengthOf(splice[0]!)).toBe(12)
    expect(totalLength(splice)).toBe(20)
  })

  it('is zero for a splice with nothing in it', () => {
    expect(totalLength([])).toBe(0)
  })
})

describe('bandsOf', () => {
  it('places a stretch where it sits on the donor, not scaled to fit', () => {
    // 10..22 of a 200-second video is 5% in, 6% wide — two marks near the
    // start, which is what the person should see.
    const [band] = bandsOf([cut({ starts_at: 10, ends_at: 22 })], 200)!
    expect(band!.left).toBeCloseTo(0.05)
    expect(band!.width).toBeCloseTo(0.06)
  })

  it('draws nothing without a length, rather than drawing a guess', () => {
    const splice = [cut({ starts_at: 10, ends_at: 22 })]
    expect(bandsOf(splice, null)).toBeNull()
    expect(bandsOf(splice, 0)).toBeNull()
    expect(bandsOf(splice, Number.NaN)).toBeNull()
  })

  it('keeps a stretch inside the track when the donor was shortened later', () => {
    // The cut was made against a 200-second video; the length has since been
    // corrected to 15. The band must not run off the end of the row.
    const [band] = bandsOf([cut({ starts_at: 10, ends_at: 22 })], 15)!
    expect(band!.left).toBeCloseTo(10 / 15)
    expect(band!.left + band!.width).toBeLessThanOrEqual(1)
  })
})

describe('tracksOf', () => {
  it('groups by the video, donors in the order they first appear', () => {
    const tracks = tracksOf([
      cut({ id: 'a', source_id: 's1', source_title: 'First', starts_at: 10, ends_at: 20 }),
      cut({ id: 'b', source_id: 's2', source_title: 'Second', starts_at: 5, ends_at: 9 }),
      cut({ id: 'c', source_id: 's1', source_title: 'First', starts_at: 90, ends_at: 99 }),
    ])
    expect(tracks.map((track) => track.source_id)).toEqual(['s1', 's2'])
    expect(tracks[0]!.cuts.map((one) => one.id)).toEqual(['a', 'c'])
    expect(tracks[1]!.cuts).toHaveLength(1)
  })
})

describe('blockerOf', () => {
  it('tells an empty splice from one whose donor has no video yet', () => {
    expect(blockerOf([])).toBe('empty')
    expect(blockerOf([shot({ path: null })])).toBe('noFile')
    expect(blockerOf([shot(), shot({ cut_id: 'c2', path: null })])).toBe('noFile')
    expect(blockerOf([shot()])).toBeNull()
  })
})

describe('canBeCut', () => {
  it('follows the facts: a donor, or stretches already taken', () => {
    expect(canBeCut([], 0)).toBe(false)
    expect(canBeCut([], 1)).toBe(true)
    // Stretches with no donor link left: the data is there, so the screen is.
    expect(canBeCut([cut({ starts_at: 1, ends_at: 2 })], 0)).toBe(true)
  })
})

describe('orderMoving', () => {
  const splice = [
    cut({ id: 'a', starts_at: 1, ends_at: 2 }),
    cut({ id: 'b', starts_at: 3, ends_at: 4 }),
    cut({ id: 'c', starts_at: 5, ends_at: 6 }),
  ]

  it('puts the moved stretch before the one it was dropped on', () => {
    expect(orderMoving(splice, 'c', 'a')).toEqual(['c', 'a', 'b'])
    expect(orderMoving(splice, 'a', 'c')).toEqual(['b', 'a', 'c'])
  })

  it('puts it last when dropped past the end', () => {
    expect(orderMoving(splice, 'a', null)).toEqual(['b', 'c', 'a'])
  })

  it('names every stretch exactly once, whatever it is handed', () => {
    // The backend refuses a list that names one twice or leaves one out, so a
    // drag onto itself or onto something gone must still produce a whole list.
    expect(orderMoving(splice, 'b', 'b')).toEqual(['a', 'c', 'b'])
    expect(orderMoving(splice, 'b', 'gone')).toEqual(['a', 'c', 'b'])
  })
})
