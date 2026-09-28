import { describe, expect, it } from 'vitest'
import type { Cut, Shot } from '@/lib/api/types'
import {
  SHORTEST_DRAG,
  blockerOf,
  canBeCut,
  dragged,
  lengthOf,
  orderMoving,
  orderWithin,
  scaleOf,
  totalLength,
  tracksOf,
} from '@/lib/cuts'

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
    const splice = [
      cut({ starts_at: 10, ends_at: 22 }),
      cut({ id: 'c2', starts_at: 90, ends_at: 98 }),
    ]
    expect(lengthOf(splice[0]!)).toBe(12)
    expect(totalLength(splice)).toBe(20)
  })

  it('is zero for a splice with nothing in it', () => {
    expect(totalLength([])).toBe(0)
  })
})

// Where a stretch sits on the track, and that it stays inside it, is dowel's
// Track since v0.80 (`track-segments`, tested there). What stays here is the
// refusal to draw against a length nobody knows.
describe('scaleOf', () => {
  it('draws against the donor length it is given', () => {
    expect(scaleOf(200)).toBe(200)
  })

  it('draws nothing without a length, rather than drawing a guess', () => {
    expect(scaleOf(null)).toBeNull()
    expect(scaleOf(0)).toBeNull()
    expect(scaleOf(Number.NaN)).toBeNull()
  })
})

describe('dragged', () => {
  const stretch = { starts_at: 10, ends_at: 22 }

  it('moves the whole stretch and keeps its length', () => {
    expect(dragged(stretch, 'whole', 5, 200)).toEqual({ starts_at: 15, ends_at: 27 })
    expect(dragged(stretch, 'whole', -4, 200)).toEqual({ starts_at: 6, ends_at: 18 })
  })

  it('stops the whole stretch at either end of the donor', () => {
    expect(dragged(stretch, 'whole', -50, 200)).toEqual({ starts_at: 0, ends_at: 12 })
    expect(dragged(stretch, 'whole', 500, 200)).toEqual({ starts_at: 188, ends_at: 200 })
  })

  it('moves one end and leaves the other where it was', () => {
    expect(dragged(stretch, 'start', -3, 200)).toEqual({ starts_at: 7, ends_at: 22 })
    expect(dragged(stretch, 'end', 8, 200)).toEqual({ starts_at: 10, ends_at: 30 })
  })

  it('stops an end short of the other one, and at the edge of the donor', () => {
    expect(dragged(stretch, 'start', 40, 200)).toEqual({
      starts_at: 22 - SHORTEST_DRAG,
      ends_at: 22,
    })
    expect(dragged(stretch, 'end', -40, 200)).toEqual({
      starts_at: 10,
      ends_at: 10 + SHORTEST_DRAG,
    })
    expect(dragged(stretch, 'start', -40, 200).starts_at).toBe(0)
    expect(dragged(stretch, 'end', 400, 200).ends_at).toBe(200)
  })

  it('steps in tenths, and a press that barely moved changes nothing', () => {
    expect(dragged(stretch, 'whole', 0.04, 200)).toBe(stretch)
    expect(dragged(stretch, 'end', 1.26, 200)).toEqual({ starts_at: 10, ends_at: 23.3 })
    // Three tenths added one at a time are not 0.30000000000000004 in store.
    expect(dragged({ starts_at: 0.1, ends_at: 1 }, 'start', 0.2, 200).starts_at).toBe(0.3)
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
    expect(canBeCut(0, 0)).toBe(false)
    expect(canBeCut(0, 1)).toBe(true)
    // Stretches with no donor link left: the data is there, so the screen is.
    expect(canBeCut(1, 0)).toBe(true)
    // A donor only ever cut from: its tab names what was taken.
    expect(canBeCut(0, 0, 2)).toBe(true)
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

describe('orderWithin', () => {
  // Two donors interleaved: a and c from the first, b and d from the second.
  const a = cut({ id: 'a', source_id: 's1', starts_at: 1, ends_at: 2 })
  const b = cut({ id: 'b', source_id: 's2', starts_at: 1, ends_at: 2 })
  const c = cut({ id: 'c', source_id: 's1', starts_at: 5, ends_at: 6 })
  const d = cut({ id: 'd', source_id: 's2', starts_at: 5, ends_at: 6 })
  const splice = [a, b, c, d]

  it('puts a stretch before the one that follows it in its own list', () => {
    expect(orderWithin(splice, [a, c], 'c', 0)).toEqual(['c', 'a', 'b', 'd'])
  })

  it('moves a stretch to the bottom of its list without leaving its donor', () => {
    // Just after c, the first donor's last - not after d, which is the second
    // donor's and happens to end the splice.
    expect(orderWithin(splice, [a, c], 'a', 1)).toEqual(['b', 'c', 'a', 'd'])
    expect(orderWithin(splice, [b, d], 'b', 1)).toEqual(['a', 'c', 'd', 'b'])
  })

  it('names every stretch exactly once', () => {
    expect([...orderWithin(splice, [a], 'a', 0)].sort()).toEqual(['a', 'b', 'c', 'd'])
  })
})
