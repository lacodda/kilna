import { describe, expect, it } from 'vitest'
import type { ScheduledRelease, ScoredWork } from '@/lib/api'
import { SHORTLIST_LIMIT, isQuiet, summarise } from '@/lib/dashboard'

const TODAY = '2026-08-27'

function work(over: Partial<ScoredWork> = {}): ScoredWork {
  return {
    work_id: 'w1',
    title: 'Harbour lights',
    kind: 'song',
    status: 'draft',
    total: null,
    tier: null,
    scored_at: null,
    stale: false,
    released: 0,
    scheduled: 0,
    updated_at: '2026-08-26T10:00:00Z',
    ...over,
  }
}

function release(over: Partial<ScheduledRelease> = {}): ScheduledRelease {
  return {
    id: 'r1',
    work_id: 'w1',
    kind: 'single',
    status: 'planned',
    title: null,
    scheduled_at: '2026-08-29',
    released_at: null,
    url: null,
    slot_pinned_at: null,
    meta: {},
    created_at: TODAY,
    updated_at: TODAY,
    work_title: 'Harbour lights',
    total: 30,
    tier: 'clip',
    readiness: { roles: [], scored: true, ready: true },
    ...over,
  }
}

const UNREADY = { roles: [], scored: false, ready: false }

describe('summarise', () => {
  it('puts an unready release with a near date among the decisions', () => {
    const summary = summarise([], [release({ readiness: UNREADY })], TODAY)

    expect(summary.decisions).toHaveLength(1)
    expect(summary.decisions[0]?.daysLeft).toBe(2)
  })

  it('leaves a ready release out of the decisions but keeps it in the week', () => {
    const summary = summarise([], [release()], TODAY)

    expect(summary.decisions).toHaveLength(0)
    expect(summary.week).toHaveLength(1)
  })

  /** The rule the catalogue's gaps already carry: only what is still open. */
  it('says nothing about a release that has already gone out', () => {
    const summary = summarise(
      [],
      [release({ released_at: '2026-08-20', scheduled_at: '2026-08-20', readiness: UNREADY })],
      TODAY,
    )

    expect(summary.decisions).toHaveLength(0)
    expect(summary.week).toHaveLength(0)
  })

  it('counts an overdue unready release as the most pressing decision', () => {
    const summary = summarise(
      [],
      [
        release({ id: 'soon', scheduled_at: '2026-08-30', readiness: UNREADY }),
        release({ id: 'late', scheduled_at: '2026-08-25', readiness: UNREADY }),
      ],
      TODAY,
    )

    expect(summary.decisions.map((d) => d.release.id)).toEqual(['late', 'soon'])
    expect(summary.decisions[0]?.daysLeft).toBe(-2)
  })

  it('keeps a far-off gap out of the decisions until its week comes', () => {
    const summary = summarise([], [release({ scheduled_at: '2026-10-01', readiness: UNREADY })], TODAY)

    expect(summary.decisions).toHaveLength(0)
  })

  it('ignores a release with no date at all', () => {
    const summary = summarise([], [release({ scheduled_at: null, readiness: UNREADY })], TODAY)

    expect(summary.decisions).toHaveLength(0)
    expect(summary.week).toHaveLength(0)
  })

  it('shortlists scored work that is going nowhere, strongest first', () => {
    const summary = summarise(
      [
        work({ work_id: 'weak', title: 'Weak', total: 10 }),
        work({ work_id: 'strong', title: 'Strong', total: 40 }),
      ],
      [],
      TODAY,
    )

    expect(summary.shortlist.map((w) => w.work_id)).toEqual(['strong', 'weak'])
  })

  it('keeps work that is already booked or already out off the shortlist', () => {
    const summary = summarise(
      [
        work({ work_id: 'booked', total: 40, scheduled: 1 }),
        work({ work_id: 'shipped', total: 40, released: 1 }),
      ],
      [],
      TODAY,
    )

    expect(summary.shortlist).toHaveLength(0)
  })

  it('stops the shortlist at a length that is still a shortlist', () => {
    const many = Array.from({ length: SHORTLIST_LIMIT + 4 }, (_, index) =>
      work({ work_id: `w${index}`, title: `Work ${index}`, total: 50 - index }),
    )

    expect(summarise(many, [], TODAY).shortlist).toHaveLength(SHORTLIST_LIMIT)
  })

  it('lists unscored work, and not the unscored thing already released', () => {
    const summary = summarise(
      [
        work({ work_id: 'fresh', title: 'Fresh' }),
        work({ work_id: 'gone', title: 'Gone', released: 1 }),
      ],
      [],
      TODAY,
    )

    expect(summary.unscored.map((w) => w.work_id)).toEqual(['fresh'])
  })
})

describe('isQuiet', () => {
  it('is quiet with nothing at all', () => {
    expect(isQuiet(summarise([], [], TODAY))).toBe(true)
  })

  /** A finished workspace is quiet, and that is the screen working. */
  it('is quiet when everything is scored, booked and shipped', () => {
    const summary = summarise(
      [work({ total: 40, released: 1, scheduled: 1 })],
      [release({ released_at: '2026-08-20' })],
      TODAY,
    )

    expect(isQuiet(summary)).toBe(true)
  })

  it('is not quiet while one thing still needs something', () => {
    expect(isQuiet(summarise([work()], [], TODAY))).toBe(false)
  })
})
