import { describe, expect, it } from 'vitest'
import type { Dismissal, ProfileConfig, ScheduledRelease, ScoredWork } from '@/lib/api/types'
import {
  NO_LINK_AFTER_DAYS,
  STALE_DRAFT_DAYS,
  answeredOn,
  dismissalKey,
  findings,
  identify,
  rhythmGaps,
  visible,
  type Finding,
  type FindingKind,
} from '@/lib/findings'

const TODAY = '2026-08-28'

const CONFIG: Pick<ProfileConfig, 'work_kinds' | 'prompts'> = {
  work_kinds: [
    {
      key: 'song',
      label: 'Song',
      axes: [{ key: 'hook', label: 'Hook', kind: 'scale', weight: 1, scale: 10 }],
      tiers: [
        { key: 'hold', label: 'Hold', min: 0 },
        { key: 'audio', label: 'Audio', min: 55 },
        { key: 'clip', label: 'Clip', min: 78 },
      ],
    },
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
    created_at: '2026-08-15T10:00:00Z',
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
    scheduled_at: '2026-09-05',
    released_at: null,
    url: null,
    slot_pinned_at: null,
    scheduled_time: null,
    time_zone: null,
    meta: {},
    created_at: TODAY,
    updated_at: TODAY,
    work_title: 'Harbour lights',
    total: 20,
    tier: 'hold',
    readiness: { checks: [], ready: true },
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
  // An audio release goes out for a song that was judged; it has no axes of
  // its own, and chasing it for a score would put every one on the board.
  it('says nothing about a work of a kind nobody judges', () => {
    const audio = work({ kind: 'audio', total: null, tier: null, scored_at: null })
    const withAudio = {
      ...CONFIG,
      work_kinds: [...CONFIG.work_kinds, { key: 'audio', label: 'Audio' }],
    }

    expect(findings([audio], [], withAudio, TODAY)).toEqual([])
  })

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
    const found = findings([work({ updated_at: daysAgo(STALE_DRAFT_DAYS + 1) })], [], CONFIG, TODAY)

    expect(kinds(found)).toContain('stale-draft')
    expect(found.find((f) => f.kind === 'stale-draft')?.action).toBe('polish')
  })

  it('leaves a draft alone while it is still being worked on', () => {
    const found = findings([work({ updated_at: daysAgo(STALE_DRAFT_DAYS - 1) })], [], CONFIG, TODAY)

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
      const found = findings([stronger], [release({ released_at: '2026-08-20' })], CONFIG, TODAY)

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
      const next = findings(
        [work({ updated_at: daysAgo(STALE_DRAFT_DAYS + 2) })],
        [],
        CONFIG,
        TODAY,
      )

      const complaint = (list: ReturnType<typeof findings>) =>
        list.find((f) => f.kind === 'stale-draft')?.complaint

      expect(complaint(day)).toBe(complaint(next))
    })
  })

  it('offers no action a profile does not have', () => {
    const bare = { work_kinds: CONFIG.work_kinds, prompts: [] }
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

describe('dismissing a complaint', () => {
  const found = (over: Partial<ScoredWork> = {}) => findings([work(over)], [], CONFIG, TODAY)

  const dismissal = (finding: Finding): Dismissal => ({
    ...dismissalKey(finding),
    dismissed_at: '2026-08-28T09:00:00Z',
  })

  /** The one finding of this kind, insisting there is exactly one. */
  const only = (list: readonly Finding[], kind: FindingKind): Finding => {
    const matching = list.filter((finding) => finding.kind === kind)
    expect(matching).toHaveLength(1)
    return matching[0] as Finding
  }

  it('hides the complaint that was answered', () => {
    const standing = found({ total: null })

    expect(visible(standing, [dismissal(only(standing, 'unscored'))])).toEqual([])
  })

  /**
   * The point of the stage. Hiding answers *this* complaint, not the work: a
   * draft that has now sat four months is a different thing to hear than one
   * that had sat one, and it has to come back. The predecessor hid by work and
   * went quiet about it for good.
   */
  it('raises the same work again once its complaint changes', () => {
    const oneMonth = found({ updated_at: daysAgo(35), scheduled: 0 })
    const fourMonths = found({ updated_at: daysAgo(125), scheduled: 0 })
    const stale = (list: readonly Finding[]) => list.filter((f) => f.kind === 'stale-draft')

    const answered = [dismissal(only(oneMonth, 'stale-draft'))]

    expect(stale(visible(oneMonth, answered))).toEqual([])
    expect(stale(visible(fourMonths, answered))).toHaveLength(1)
  })

  it('leaves a different kind about the same work alone', () => {
    const standing = findings([work({ stale: true, scheduled: 0 })], [], CONFIG, TODAY)
    const answered = [dismissal(only(standing, 'stale-score'))]

    expect(kinds(visible(standing, answered))).toEqual(['ready-unscheduled'])
  })

  it('leaves the same complaint about a different work alone', () => {
    const standing = findings(
      [
        work({ work_id: 'a', title: 'Alpha', total: null }),
        work({ work_id: 'b', title: 'Beta', total: null }),
      ],
      [],
      CONFIG,
      TODAY,
    )
    const alpha = standing.find((finding) => finding.workId === 'a') as Finding
    const answered = [dismissal(alpha)]

    const left = visible(standing, answered)
    expect(left).toHaveLength(1)
    expect(left[0]?.workId).toBe('b')
  })

  it('is unmoved by a dismissal for something that is no longer complained about', () => {
    const standing = found({ total: null })
    const stale: Dismissal = {
      kind: 'stale-draft',
      work_id: 'gone',
      complaint: 'stale-draft:2',
      dismissed_at: '2026-01-01T00:00:00Z',
    }

    expect(visible(standing, [stale])).toEqual(standing)
  })
})

/**
 * The dismissal key carries the kind as well as the complaint, and every
 * complaint string happens to start with its kind — so the kind looks
 * redundant. It is not: nothing stops a future finding from phrasing its
 * complaint some other way, and the moment two kinds phrase one the same,
 * dismissing one would silence the other. This is the guarantee that the key
 * does not lean on that coincidence.
 */
describe('the dismissal key', () => {
  it('separates two kinds that say the same thing about one work', () => {
    const first: Finding = {
      kind: 'unscored',
      workId: 'w1',
      title: 'Harbour lights',
      complaint: 'same wording',
    }
    const second: Finding = { ...first, kind: 'stale-draft' }

    const answered: Dismissal[] = [{ ...dismissalKey(first), dismissed_at: '2026-08-28T09:00:00Z' }]

    expect(kinds(visible([first, second], answered))).toEqual(['stale-draft'])
  })
})

describe('identify', () => {
  it('tells two complaints about one work apart, and one complaint from itself not', () => {
    const first: Finding = { kind: 'stale-score', workId: 'w1', title: 'A', complaint: 'x' }

    expect(identify(first)).toBe(identify({ ...first, title: 'renamed' }))
    expect(identify(first)).not.toBe(identify({ ...first, complaint: 'y' }))
    expect(identify(first)).not.toBe(identify({ ...first, kind: 'stale-draft' }))
  })
})

describe('answeredOn', () => {
  it('opens the tab each complaint is answered on, and none for a stalled draft', () => {
    const tabs = (
      ['unscored', 'stale-score', 'ready-unscheduled', 'weak-scheduled', 'stale-draft'] as const
    ).map(answeredOn)

    expect(tabs).toEqual(['score', 'score', 'overview', 'overview', undefined])
  })
})

describe('findings after the day (v0.97)', () => {
  // A released work, so only the release's own complaints are in play.
  const out = work({ released: 1, scheduled: 0 })
  const after = (entry: ScheduledRelease) =>
    findings([out], [entry], CONFIG, TODAY).filter(
      (finding) => finding.kind === 'no-link' || finding.kind === 'unmarked',
    )

  it('says a release out for days with no link, and not the day it went', () => {
    const wentOut = (days: number, url: string | null) =>
      release({
        status: 'released',
        scheduled_at: daysAgo(days).slice(0, 10),
        released_at: daysAgo(days),
        url,
      })

    expect(after(wentOut(NO_LINK_AFTER_DAYS, null)).map((f) => f.kind)).toEqual(['no-link'])
    expect(after(wentOut(NO_LINK_AFTER_DAYS - 1, null))).toEqual([])
    expect(after(wentOut(30, 'https://youtu.be/abc'))).toEqual([])
    expect(after(wentOut(30, '   ')).map((f) => f.kind)).toEqual(['no-link'])
  })

  it('says a planned release whose day has passed, from the day after it', () => {
    expect(after(release({ scheduled_at: TODAY }))).toEqual([])
    const late = after(release({ scheduled_at: daysAgo(1).slice(0, 10) }))
    expect(late.map((f) => f.kind)).toEqual(['unmarked'])
    // The day is part of the complaint: moved to another day that passes,
    // it is news again after a dismissal.
    expect(late[0]?.complaint).toBe(`unmarked:r1:${daysAgo(1).slice(0, 10)}`)
    // One with a link written and no mark is the same complaint: the link is
    // what the mark will read.
    expect(
      after(release({ scheduled_at: daysAgo(3).slice(0, 10), url: 'https://youtu.be/x' })).map(
        (f) => f.kind,
      ),
    ).toEqual(['unmarked'])
  })

  it('says nothing of a release in the queue', () => {
    expect(after(release({ scheduled_at: null }))).toEqual([])
  })

  it('sends both to the overview, where the release is written and marked', () => {
    expect(answeredOn('no-link')).toBe('overview')
    expect(answeredOn('unmarked')).toBe('overview')
  })
})

describe('a month short of the rhythm (v0.104)', () => {
  // 28 August: three days left of it, from tomorrow, and all of September.
  it('says how many releases each month is short, and how many wait', () => {
    const calendar = [
      release({ id: 'a', scheduled_at: '2026-09-02' }),
      release({ id: 'b', scheduled_at: '2026-09-10', status: 'released' }),
      release({ id: 'c', scheduled_at: '2026-08-20', status: 'released' }),
    ]

    const found = rhythmGaps(calendar, 7, 3, TODAY)

    expect(found.map((f) => [f.complaint, f.params?.count, f.params?.queued])).toEqual([
      ['rhythm-gap:2026-08:1', 1, 7],
      ['rhythm-gap:2026-09:8', 8, 7],
    ])
    expect(found[1]!.to).toEqual({ path: '/calendar', day: '2026-09-01' })
    expect(found[1]!.workId, 'about the plan, not a work').toBe('')
  })

  it('says nothing of a full month, or without a rhythm', () => {
    const full = Array.from({ length: 10 }, (_, i) =>
      release({ id: `r${i}`, scheduled_at: `2026-09-${String(i * 3 + 1).padStart(2, '0')}` }),
    )
    const gaps = rhythmGaps(
      [...full, release({ id: 'aug', scheduled_at: '2026-08-30' })],
      0,
      3,
      TODAY,
    )
    expect(gaps).toEqual([])
    expect(rhythmGaps([], 3, null, TODAY)).toEqual([])
  })
})
