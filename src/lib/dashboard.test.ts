import { describe, expect, it } from 'vitest'
import type { ProfileConfig, ScheduledRelease, ScoredWork } from '@/lib/api/types'
import {
  NEAREST_LIMIT,
  SCHEDULE_LIMIT,
  aside,
  decide,
  isQuiet,
  subjectOf,
  summarise,
  tally,
} from '@/lib/dashboard'
import { findings, type Finding } from '@/lib/findings'

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
    created_at: '2026-08-20T10:00:00Z',
    collection_id: null,
    tags: [],
    marks: [],
    tier_pinned: false,
    version_count: 0,
    stage: null,
    bookmarked_at: null,
    publications: [],
    ...over,
  }
}

function release(over: Partial<ScheduledRelease> = {}): ScheduledRelease {
  return {
    id: 'r1',
    work_id: 'w1',
    work_kind: 'song',
    work_stage: null,
    kind: 'single',
    status: 'planned',
    title: null,
    scheduled_at: '2026-08-29',
    released_at: null,
    url: null,
    slot_pinned_at: null,
    scheduled_time: null,
    time_zone: null,
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
    const summary = summarise(
      [],
      [release({ scheduled_at: '2026-10-01', readiness: UNREADY })],
      TODAY,
    )

    expect(summary.decisions).toHaveLength(0)
  })

  it('ignores a release with no date at all', () => {
    const summary = summarise([], [release({ scheduled_at: null, readiness: UNREADY })], TODAY)

    expect(summary.decisions).toHaveLength(0)
    expect(summary.week).toHaveLength(0)
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

  /** The audit of 24.09: the section showed scored work with nothing booked. */
  it('reads the nearest slots as what goes out next, ready or not', () => {
    const summary = summarise(
      [work({ work_id: 'unbooked', total: 90 })],
      [
        release({ id: 'later', scheduled_at: '2026-09-20' }),
        release({ id: 'soon', scheduled_at: '2026-08-29', readiness: UNREADY }),
        release({ id: 'undated', scheduled_at: null }),
      ],
      TODAY,
    )

    expect(summary.nearest.map((slot) => slot.release.id)).toEqual(['soon', 'later'])
  })

  it('leaves what went out and what is past its date out of the nearest slots', () => {
    const summary = summarise(
      [],
      [
        release({ id: 'out', scheduled_at: '2026-08-29', released_at: '2026-08-29' }),
        release({ id: 'late', scheduled_at: '2026-08-20' }),
      ],
      TODAY,
    )

    expect(summary.nearest).toHaveLength(0)
  })

  it('stops the nearest slots at a row of covers', () => {
    const many = Array.from({ length: NEAREST_LIMIT + 3 }, (_, index) =>
      release({ id: `r${index}`, scheduled_at: `2026-09-${String(10 + index)}` }),
    )

    expect(summarise([], many, TODAY).nearest).toHaveLength(NEAREST_LIMIT)
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

const CONFIG: Pick<ProfileConfig, 'work_kinds' | 'prompts'> = {
  work_kinds: [
    {
      key: 'song',
      label: 'Song',
      axes: [{ key: 'hook', label: 'Hook', kind: 'scale', weight: 1, scale: 10 }],
    },
  ],
  prompts: [{ key: 'score', label: 'Score it', description: '', template: '', produces: 'score' }],
}

/** The decisions a workspace comes to, read the way the screen reads it. */
function decisionsOf(works: ScoredWork[], calendar: ScheduledRelease[] = []) {
  const summary = summarise(works, calendar, TODAY)
  const standing = findings(works, calendar, CONFIG, TODAY)
  return { decisions: decide(summary, standing, works), standing }
}

const SCORED = { total: 40, scored_at: '2026-08-20T10:00:00Z' }

describe('decide', () => {
  it('sends a release nothing has scored to the score, one missing a draft to the versions', () => {
    const { decisions } = decisionsOf(
      [],
      [
        release({ id: 'unscored', work_id: 'a', work_title: 'A', readiness: UNREADY }),
        release({
          id: 'no-lyrics',
          work_id: 'b',
          work_title: 'B',
          scheduled_at: '2026-08-28',
          readiness: { roles: [{ role: 'lyrics', present: false }], scored: true, ready: false },
        }),
      ],
    )

    expect(decisions.map((d) => [d.key, d.move, d.tab])).toEqual([
      ['release:no-lyrics', 'open', 'versions'],
      ['release:unscored', 'score', 'score'],
    ])
    expect(decisions[0]?.kind === 'release' && decisions[0].gaps).toEqual(['lyrics'])
  })

  it('puts a stale score forward to be re-scored', () => {
    const { decisions } = decisionsOf([work({ ...SCORED, stale: true, scheduled: 1 })])

    expect(decisions.map((d) => [d.move, d.tab])).toEqual([['rescore', 'score']])
  })

  it('puts judged work with nothing booked forward to be scheduled, strongest first', () => {
    const { decisions } = decisionsOf([
      work({ work_id: 'weak', title: 'Weak', ...SCORED, total: 10 }),
      work({ work_id: 'strong', title: 'Strong', ...SCORED, total: 80 }),
    ])

    expect(decisions.map((d) => [d.workId, d.move, d.tab])).toEqual([
      ['strong', 'schedule', 'releases'],
      ['weak', 'schedule', 'releases'],
    ])
    expect(decisions[0]?.kind === 'finding' && decisions[0].total).toBe(80)
  })

  it('stops putting work forward at a length that is still a decision', () => {
    const many = Array.from({ length: SCHEDULE_LIMIT + 4 }, (_, index) =>
      work({ work_id: `w${index}`, title: `Work ${index}`, ...SCORED, total: 50 - index }),
    )

    expect(decisionsOf(many).decisions).toHaveLength(SCHEDULE_LIMIT)
  })

  it('draws one card per work: the date first, then the score, then the slot', () => {
    const { decisions } = decisionsOf(
      [
        work({ work_id: 'booked', title: 'Booked', ...SCORED, stale: true, scheduled: 1 }),
        work({ work_id: 'free', title: 'Free', ...SCORED, stale: true }),
      ],
      [release({ id: 'r-booked', work_id: 'booked', work_title: 'Booked', readiness: UNREADY })],
    )

    expect(decisions.map((d) => [d.workId, d.move])).toEqual([
      ['booked', 'score'],
      ['free', 'rescore'],
    ])
  })

  it('draws nothing for a complaint the person has dismissed', () => {
    const works = [work({ ...SCORED })]

    expect(decide(summarise(works, [], TODAY), [], works)).toEqual([])
  })
})

describe('aside', () => {
  it('leaves out what a card or the lead column already says', () => {
    const { decisions, standing } = decisionsOf([
      work({ work_id: 'unscored', title: 'Unscored', total: null }),
      work({ work_id: 'stale', title: 'Stale', ...SCORED, stale: true, scheduled: 1 }),
      work({
        work_id: 'stalled',
        title: 'Stalled',
        ...SCORED,
        updated_at: '2026-06-01T10:00:00Z',
      }),
    ])

    expect(aside(standing, decisions).map((f: Finding) => [f.kind, f.workId])).toEqual([
      ['stale-draft', 'stalled'],
    ])
  })

  /** Said once, somewhere: the stale score a release card displaced is not lost. */
  it('keeps the complaint a release card displaced', () => {
    const { decisions, standing } = decisionsOf(
      [work({ ...SCORED, stale: true, scheduled: 1 })],
      [release({ readiness: UNREADY })],
    )

    expect(aside(standing, decisions).map((f) => f.kind)).toEqual(['stale-score'])
  })
})

describe('tally', () => {
  it('counts the catalogue, what is judged, what is not, and what went out', () => {
    expect(
      tally([
        work({ work_id: 'a', ...SCORED }),
        work({ work_id: 'b', ...SCORED, released: 2 }),
        work({ work_id: 'c' }),
        work({ work_id: 'd', released: 1 }),
      ]),
    ).toEqual({ works: 4, scored: 2, unscored: 1, released: 2 })
  })
})

describe('subjectOf', () => {
  it('reads the action and the work back out of a task key', () => {
    expect(subjectOf('score:w1')).toEqual({ action: 'score', workId: 'w1' })
    expect(subjectOf('prompts:w1:sc1:still')).toEqual({ action: 'prompts', workId: 'w1' })
  })

  it('reads no work out of a task about a comment, a screenshot or a release', () => {
    expect(subjectOf('reply:comment:c1')).toEqual({ action: 'reply', workId: null })
    expect(subjectOf('read:channel:yt:p1')).toEqual({ action: 'read', workId: null })
    expect(subjectOf('release-meta:release:r1')).toEqual({ action: 'release-meta', workId: null })
    // A board's task is about its publication, named third.
    expect(subjectOf('cover-ideas:cover:w1')).toEqual({ action: 'cover-ideas', workId: 'w1' })
  })
})
