import { describe, expect, it } from 'vitest'
import type { ProfileConfig, ScheduledRelease, ScoredWork } from '@/lib/api'
import { STALE_DRAFT_DAYS, findings, type FindingKind } from '@/lib/findings'

const TODAY = '2026-08-28'

const CONFIG: Pick<ProfileConfig, 'tiers' | 'prompts'> = {
  tiers: [
    { key: 'hold', label: 'Hold', min: 0 },
    { key: 'audio', label: 'Audio', min: 55 },
    { key: 'clip', label: 'Clip', min: 78 },
  ],
  prompts: [
    { key: 'score', label: 'Score it', description: '', template: '', produces: 'score' },
    { key: 'polish', label: 'Suggest a revision', description: '', template: '' },
  ],
}

function work(over: Partial<ScoredWork> = {}): ScoredWork {
  return {
    work_id: 'w1',
    title: 'Harbour lights',
    kind: 'song',
    status: 'draft',
    total: 40,
    tier: 'hold',
    scored_at: '2026-08-20T10:00:00Z',
    stale: false,
    released: 0,
    scheduled: 0,
    updated_at: '2026-08-27T10:00:00Z',
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
    scheduled_at: '2026-09-05',
    released_at: null,
    url: null,
    slot_pinned_at: null,
    meta: {},
    created_at: TODAY,
    updated_at: TODAY,
    work_title: 'Harbour lights',
    total: 20,
    tier: 'hold',
    readiness: { roles: [], scored: true, ready: true },
    ...over,
  }
}

/** A day that many days before today, as an ISO timestamp. */
function daysAgo(days: number): string {
  const ms = Date.UTC(2026, 7, 28) - days * 86_400_000
  return new Date(ms).toISOString()
}

const kinds = (list: { kind: FindingKind }[]) => list.map((f) => f.kind).sort()

describe('findings', () => {
  it('says an unjudged work is unscored, and offers the scoring action', () => {
    const found = findings([work({ total: null, tier: null, scored_at: null })], [], CONFIG, TODAY)

    expect(kinds(found)).toContain('unscored')
    expect(found.find((f) => f.kind === 'unscored')?.action).toBe('score')
  })

  /** The predecessor's mistake, kept as a test: never chase finished work. */
  it('says nothing at all about work that has already gone out', () => {
    const found = findings(
      [work({ total: null, released: 1 }), work({ work_id: 'w2', stale: true, released: 1 })],
      [],
      CONFIG,
      TODAY,
    )

    expect(found).toEqual([])
  })

  it('does not also chase an unscored work for having no release booked', () => {
    const found = findings([work({ total: null, tier: null, scored_at: null })], [], CONFIG, TODAY)

    expect(kinds(found)).not.toContain('ready-unscheduled')
  })

  it('notices a score that describes an older draft', () => {
    const found = findings([work({ stale: true })], [], CONFIG, TODAY)

    expect(kinds(found)).toContain('stale-score')
  })

  it('notices judged work with nothing booked', () => {
    const found = findings([work()], [], CONFIG, TODAY)

    expect(kinds(found)).toContain('ready-unscheduled')
  })

  it('leaves booked work alone', () => {
    const found = findings([work({ scheduled: 1 })], [], CONFIG, TODAY)

    expect(kinds(found)).not.toContain('ready-unscheduled')
  })

  it('notices a draft nobody has touched for a month', () => {
    const found = findings(
      [work({ updated_at: daysAgo(STALE_DRAFT_DAYS + 1) })],
      [],
      CONFIG,
      TODAY,
    )

    expect(kinds(found)).toContain('stale-draft')
    expect(found.find((f) => f.kind === 'stale-draft')?.action).toBe('polish')
  })

  it('leaves a draft alone while it is still being worked on', () => {
    const found = findings(
      [work({ updated_at: daysAgo(STALE_DRAFT_DAYS - 1) })],
      [],
      CONFIG,
      TODAY,
    )

    expect(kinds(found)).not.toContain('stale-draft')
  })

  it('does not call a booked work stalled — it is waiting for its date', () => {
    const found = findings(
      [work({ scheduled: 1, updated_at: daysAgo(STALE_DRAFT_DAYS + 10) })],
      [],
      CONFIG,
      TODAY,
    )

    expect(kinds(found)).not.toContain('stale-draft')
  })

  describe('weak-scheduled', () => {
    const stronger = work({ work_id: 'strong', title: 'Strong', total: 80, tier: 'clip' })

    it('notices the weakest tier holding a slot while stronger work waits', () => {
      const found = findings([stronger], [release()], CONFIG, TODAY)

      expect(kinds(found)).toContain('weak-scheduled')
    })

    /** With nothing better to put there, the complaint has no advice to give. */
    it('stays quiet when nothing stronger is waiting', () => {
      const found = findings([work({ scheduled: 1 })], [release()], CONFIG, TODAY)

      expect(kinds(found)).not.toContain('weak-scheduled')
    })

    it('says nothing about a slot that has already passed', () => {
      const found = findings([stronger], [release({ scheduled_at: '2026-08-01' })], CONFIG, TODAY)

      expect(kinds(found)).not.toContain('weak-scheduled')
    })

    it('says nothing about a release that already went out', () => {
      const found = findings(
        [stronger],
        [release({ released_at: '2026-08-20' })],
        CONFIG,
        TODAY,
      )

      expect(kinds(found)).not.toContain('weak-scheduled')
    })

    /** A date settled by hand is a decision, not an oversight. */
    it('leaves a pinned slot alone', () => {
      const found = findings(
        [stronger],
        [release({ slot_pinned_at: '2026-08-20T10:00:00Z' })],
        CONFIG,
        TODAY,
      )

      expect(kinds(found)).not.toContain('weak-scheduled')
    })

    it('leaves a slot held by a stronger tier alone', () => {
      const found = findings([stronger], [release({ tier: 'audio' })], CONFIG, TODAY)

      expect(kinds(found)).not.toContain('weak-scheduled')
    })
  })

  describe('the complaint string', () => {
    /** v0.34 hides by complaint: the same one stays quiet, a changed one returns. */
    it('changes when the score behind a stale finding changes', () => {
      const first = findings([work({ stale: true })], [], CONFIG, TODAY)
      const later = findings(
        [work({ stale: true, scored_at: '2026-08-25T10:00:00Z' })],
        [],
        CONFIG,
        TODAY,
      )

      const stale = (list: ReturnType<typeof findings>) =>
        list.find((f) => f.kind === 'stale-score')?.complaint

      expect(stale(first)).toBeDefined()
      expect(stale(first)).not.toBe(stale(later))
    })

    /** And does not change every morning, or hiding one would never hold. */
    it('holds steady while a stalled draft keeps sitting there', () => {
      const day = findings([work({ updated_at: daysAgo(STALE_DRAFT_DAYS + 1) })], [], CONFIG, TODAY)
      const next = findings([work({ updated_at: daysAgo(STALE_DRAFT_DAYS + 2) })], [], CONFIG, TODAY)

      const complaint = (list: ReturnType<typeof findings>) =>
        list.find((f) => f.kind === 'stale-draft')?.complaint

      expect(complaint(day)).toBe(complaint(next))
    })
  })

  it('offers no action a profile does not have', () => {
    const bare = { tiers: CONFIG.tiers, prompts: [] }
    const found = findings([work({ total: null, tier: null, scored_at: null })], [], bare, TODAY)

    expect(found.find((f) => f.kind === 'unscored')?.action).toBeUndefined()
  })

  it('returns the same order for the same workspace', () => {
    const works = [
      work({ work_id: 'b', title: 'Beta', total: null, tier: null, scored_at: null }),
      work({ work_id: 'a', title: 'Alpha', stale: true }),
    ]

    expect(findings(works, [], CONFIG, TODAY)).toEqual(findings(works, [], CONFIG, TODAY))
  })
})

/**
 * The dashboard shows its quiet state when its own four sections are empty. A
 * finding can outlive all four — a scored, booked work whose draft moved after
 * the score, with its date beyond the week — and the screen must not claim
 * nothing is waiting while that stands.
 */
describe('a finding can outlive every dashboard section', () => {
  it('reports a stale score on work that is scored, booked and far off', () => {
    const found = findings(
      [work({ scheduled: 1, stale: true })],
      [release({ scheduled_at: '2026-10-01', tier: 'audio' })],
      CONFIG,
      TODAY,
    )

    expect(kinds(found)).toEqual(['stale-score'])
  })
})
