import { describe, expect, it } from 'vitest'
import type { ProfileConfig } from '@/lib/api/types'
import { bankDataOf, bankDraftOf, bankState, plusMonths, usesOf } from '@/lib/bank'
import { dataOf, draftOf, draftReady, expectedOf, quotaFields } from '@/lib/quota'
import channel from '@/test/fixtures/profiles/track-channel.json'

const config = channel.config as unknown as ProfileConfig

describe('a rule of the quotas', () => {
  it('reads its data into boxes and writes the same data back', () => {
    const data = {
      field: 'suno_prompt',
      metric: 'char_len',
      op: 'between',
      value: [400, 500],
      scope: 'item',
      severity: 'error',
      active: true,
    }
    const draft = draftOf(data)
    expect(draft).toMatchObject({ low: 400, high: 500, severity: 'error' })
    expect(dataOf(draft)).toEqual(data)
    expect(expectedOf(draft)).toBe('400-500')
  })

  it('keeps forbidden words as a list, and a filter only when it names values', () => {
    const draft = {
      ...draftOf({}),
      field: 'title',
      metric: 'forbidden_terms' as const,
      terms: 'neon, chrome\nlofi, ',
      filterField: 'kind',
      filterEquals: ' ',
    }
    expect(dataOf(draft)).toEqual({
      field: 'title',
      metric: 'forbidden_terms',
      op: 'not_contains',
      value: ['neon', 'chrome', 'lofi'],
      scope: 'item',
      severity: 'warn',
      active: true,
    })
    const filtered = dataOf({ ...draft, filterEquals: 'track, short', filterNot: true })
    expect(filtered.filter).toEqual({ field: 'kind', equals: ['track', 'short'], not: true })
  })

  it('is ready only when it can be measured', () => {
    const base = { ...draftOf({}), field: 'title', low: 10 }
    expect(draftReady(base)).toBe(true)
    expect(draftReady({ ...base, field: ' ' })).toBe(false)
    expect(draftReady({ ...base, op: 'between', high: 5 })).toBe(false)
    expect(draftReady({ ...base, metric: 'match_count' })).toBe(false)
    expect(draftReady({ ...base, metric: 'share', scope: 'item' })).toBe(false)
    expect(draftReady({ ...base, metric: 'share', scope: 'global' })).toBe(true)
  })

  it('offers the fields the profile has: roles, overview fields and doors', () => {
    const fields = quotaFields(config)
    for (const field of [
      'title',
      'suno_prompt',
      'tradition',
      'playlists',
      'youtube.title',
      'cover',
    ])
      expect(fields, field).toContain(field)
  })
})

describe('an item of a bank', () => {
  it('rests its months after the last use, and a lock wins', () => {
    const item = { cooldown_months: 6, last_used_at: '2026-03-31' }
    expect(bankState(item, '2026-09-29')).toEqual({ state: 'cooldown', until: '2026-09' })
    expect(bankState(item, '2026-09-30')).toEqual({ state: 'available', until: null })
    expect(bankState({ cooldown_months: 6 }, '2026-01-01').state).toBe('available')
    expect(bankState({ ...item, locked: true }, '2026-04-01').state).toBe('locked')
  })

  it('adds months on the same day, or the month is shorter', () => {
    expect(plusMonths('2026-01-31', 1)).toBe('2026-02-28')
    expect(plusMonths('2026-11-15', 3)).toBe('2027-02-15')
  })

  it('keeps the uses it holds when its boxes are written', () => {
    const before = {
      group: 'West Africa',
      attributes: ['kora', 'balafon'],
      used_in: [{ work_id: 'w', title: 'Dawn', date: '2026-05-02' }],
    }
    const draft = bankDraftOf(before)
    expect(draft.attributes).toBe('kora, balafon')
    expect(bankDataOf({ ...draft, cooldown: 4, locked: true }, before)).toEqual({
      group: 'West Africa',
      attributes: ['kora', 'balafon'],
      cooldown_months: 4,
      used_in: before.used_in,
      locked: true,
    })
    expect(usesOf(before)).toEqual(before.used_in)
  })
})
