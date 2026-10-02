import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { RepeatFinding, RepeatMark } from '@/lib/api/types'
import {
  GUARD_DEFAULTS,
  guardOf,
  guardWith,
  landingWarning,
  markLabel,
  orderFindings,
  repeatHint,
  statusOf,
} from '@/lib/repeats'

/*
 * The guard of repeats as the window says it (ADR 0054): the sentence a mark
 * is explained by, the order a song's findings are read in, and the three
 * numbers of the profile's guard. Pure, so said here in both languages
 * without a window.
 */

// A day is said without its year inside the year it is read in: the clock is
// held in the fixtures' year, so the sentences below stay true next year.
beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date('2026-09-15T10:00:00Z'))
})

afterEach(() => {
  vi.useRealTimers()
})

const finding = (over: Partial<RepeatFinding> = {}): RepeatFinding => ({
  word: 'пульсар',
  level: 'red',
  why: 'rare',
  neighbour_id: 'w-tide',
  neighbour_title: 'Tide',
  day: '2026-09-20',
  booked: false,
  also: 0,
  kept: false,
  ...over,
})

const mark = (over: Partial<RepeatMark> = {}): RepeatMark => ({
  work_id: 'w-song',
  song_id: 'w-song',
  level: 'red',
  top: finding(),
  count: 1,
  ...over,
})

describe('repeatHint', () => {
  it('names the word, the other song and the day it went out', () => {
    expect(repeatHint(finding(), 'en')).toBe('пульсар — in “Tide”, out Sep 20')
  })

  it('says booked for a song that holds a day and has not gone out', () => {
    expect(repeatHint(finding({ booked: true, day: '2026-10-10' }), 'en')).toBe(
      'пульсар — in “Tide”, booked Oct 10',
    )
  })

  it('says a term of the register is spent before anything else', () => {
    const spent = finding({ word: 'кофе', level: 'orange', why: 'register' })
    expect(repeatHint(spent, 'en')).toBe('spent: кофе — in “Tide”, out Sep 20')
  })

  it('counts the other songs that say the word too', () => {
    expect(repeatHint(finding({ also: 2 }), 'en')).toBe(
      'пульсар — in “Tide”, out Sep 20, and in 2 more songs',
    )
    expect(repeatHint(finding({ also: 1 }), 'ru')).toBe(
      'пульсар — в песне «Tide», вышла 20 сент. и ещё в 1 песне',
    )
  })

  it('is said in Russian in a Russian window, the day included', () => {
    expect(repeatHint(finding(), 'ru')).toBe('пульсар — в песне «Tide», вышла 20 сент.')
    expect(repeatHint(finding({ booked: true }), 'ru')).toBe(
      'пульсар — в песне «Tide», выйдет 20 сент.',
    )
  })
})

describe('markLabel', () => {
  it('says how loud, and the loudest finding', () => {
    expect(markLabel(mark(), 'en')).toBe('Said lately: пульсар — in “Tide”, out Sep 20')
  })

  it('counts the findings behind the loudest one', () => {
    const many = mark({ level: 'orange', top: finding({ level: 'orange' }), count: 3 })
    expect(markLabel(many, 'en')).toBe('Said before: пульсар — in “Tide”, out Sep 20, and 2 more')
  })
})

describe('statusOf', () => {
  it('draws red in the bad hue and orange in the warn one', () => {
    expect(statusOf('red')).toBe('bad')
    expect(statusOf('orange')).toBe('warn')
  })
})

describe('orderFindings', () => {
  it('reads what still counts before what was kept, red before orange', () => {
    const keptRed = finding({ word: 'a', kept: true })
    const orange = finding({ word: 'b', level: 'orange' })
    const red = finding({ word: 'c' })
    const keptOrange = finding({ word: 'd', level: 'orange', kept: true })
    const redLater = finding({ word: 'e' })
    const order = orderFindings([keptRed, orange, red, keptOrange, redLater])
    expect(order.map((f) => f.word)).toEqual(['c', 'e', 'b', 'a', 'd'])
  })
})

describe('landingWarning', () => {
  it('says nothing about a day the release repeats nothing on', () => {
    expect(landingWarning([], 'en')).toBeNull()
  })

  it('names the loudest finding, and how many more', () => {
    expect(landingWarning([finding()], 'en')).toBe('Too close: пульсар — in “Tide”, out Sep 20')
    expect(landingWarning([finding(), finding({ word: 'квазар' })], 'en')).toBe(
      'Too close: пульсар — in “Tide”, out Sep 20, and 1 more',
    )
  })
})

describe('the profile guard', () => {
  it('reads the defaults when the profile names none', () => {
    expect(guardOf({ guard: null })).toEqual(GUARD_DEFAULTS)
    expect(guardOf({})).toEqual({ window_days: 90, rare_rank: 20_000, rare_in_works: 2 })
  })

  it('writes the whole guard when one number moves off its default', () => {
    expect(guardWith({ guard: null }, 'window_days', 60)).toEqual({
      window_days: 60,
      rare_rank: 20_000,
      rare_in_works: 2,
    })
  })

  it('names no guard when it is tuned back to the defaults', () => {
    const tuned = { guard: { window_days: 60, rare_rank: 20_000, rare_in_works: 2 } }
    expect(guardWith(tuned, 'window_days', 90)).toBeNull()
  })

  it('keeps every number a whole one of at least one', () => {
    expect(guardWith({ guard: null }, 'rare_in_works', 0)?.rare_in_works).toBe(1)
    expect(guardWith({ guard: null }, 'rare_rank', 2500.7)?.rare_rank).toBe(2500)
  })
})
