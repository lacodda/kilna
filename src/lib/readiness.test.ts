import { describe, expect, it } from 'vitest'
import type { TFunction } from 'i18next'
import type { Need, Readiness } from '@/lib/api/types'
import { studio } from '@/test/workspace'
import { answeredOn, daysBetween, missing, needKey, sayNeed, urgency } from './readiness'

const judged = (checks: [Need, boolean][]): Readiness => ({
  checks: checks.map(([need, done]) => ({ need, done })),
  ready: checks.every(([, done]) => done),
})

describe('urgency', () => {
  it('is calm with no date at all — the queue has no deadline', () => {
    expect(urgency(null)).toBe('calm')
  })

  it('turns at the documented bands, on the boundary days themselves', () => {
    expect(urgency(30)).toBe('calm')
    expect(urgency(8)).toBe('calm')
    expect(urgency(7)).toBe('soon')
    expect(urgency(3)).toBe('soon')
    expect(urgency(2)).toBe('urgent')
    expect(urgency(0)).toBe('urgent')
  })

  it('treats an overdue date as urgent, not as calm again', () => {
    expect(urgency(-1)).toBe('urgent')
  })
})

describe('daysBetween', () => {
  it('counts whole days between local ISO dates', () => {
    expect(daysBetween('2026-08-22', '2026-08-29')).toBe(7)
    expect(daysBetween('2026-08-22', '2026-08-22')).toBe(0)
    expect(daysBetween('2026-08-22', '2026-08-20')).toBe(-2)
  })

  it('crosses month and year edges by the calendar, not by string maths', () => {
    expect(daysBetween('2026-08-31', '2026-09-01')).toBe(1)
    expect(daysBetween('2026-12-31', '2027-01-01')).toBe(1)
    // A leap year: 2028-02 has 29 days.
    expect(daysBetween('2028-02-28', '2028-03-01')).toBe(2)
  })
})

describe('missing', () => {
  it('lists what is not done, in the order the kind asks for it', () => {
    const gaps = missing(
      judged([
        [{ kind: 'role', key: 'plot' }, true],
        [{ kind: 'score' }, false],
        [{ kind: 'cover' }, false],
        [{ kind: 'field', key: 'title' }, true],
        [{ kind: 'field', key: 'description' }, false],
      ]),
    )
    expect(gaps.map(needKey)).toEqual(['score', 'cover', 'field:description'])
  })

  it('is empty exactly when there is nothing to chase', () => {
    expect(missing(judged([[{ kind: 'role', key: 'plot' }, true]]))).toEqual([])
    expect(missing(judged([]))).toEqual([])
  })
})

describe('sayNeed', () => {
  const config = studio().profile.config
  const t = ((key: string) => `<${key}>`) as unknown as TFunction
  const youtube = { work_kind: 'video', kind: 'youtube' }

  it("says a role and a field in the profile's own words", () => {
    expect(sayNeed({ kind: 'role', key: 'plot' }, config, youtube, t)).toBe('Plot')
    expect(sayNeed({ kind: 'field', key: 'description' }, config, youtube, t)).toBe('Description')
  })

  it('says a key the profile no longer names as the key, rather than nothing', () => {
    expect(sayNeed({ kind: 'role', key: 'gone' }, config, youtube, t)).toBe('gone')
    expect(sayNeed({ kind: 'field', key: 'gone' }, config, youtube, t)).toBe('gone')
  })

  it('has a name for a checklist and a word for the missing list', () => {
    expect(sayNeed({ kind: 'cover' }, config, youtube, t, 'name')).toBe('<readiness.cover>')
    expect(sayNeed({ kind: 'cover' }, config, youtube, t)).toBe('<calendar.missingCover>')
    expect(sayNeed({ kind: 'score' }, config, youtube, t, 'name')).toBe('<readiness.score>')
    expect(sayNeed({ kind: 'score' }, config, youtube, t)).toBe('<calendar.missingScore>')
  })
})

describe('answeredOn', () => {
  const config = studio().profile.config

  it('sends each gap to the tab that closes it', () => {
    expect(answeredOn({ kind: 'role', key: 'plot' }, config, 'video')).toBe('versions')
    expect(answeredOn({ kind: 'score' }, config, 'video')).toBe('score')
    // The release is written on the overview, its one home since v0.90.
    expect(answeredOn({ kind: 'field', key: 'title' }, config, 'video')).toBe('overview')
  })

  it('sends a cover to the Cover tab of a kind that builds one, and to the files of any other', () => {
    expect(answeredOn({ kind: 'cover' }, config, 'video')).toBe('cover')
    expect(answeredOn({ kind: 'cover' }, config, 'song')).toBe('files')
  })
})
