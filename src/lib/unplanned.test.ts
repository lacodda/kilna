import { describe, expect, it } from 'vitest'
import type { ProfileConfig, ScheduledRelease, ScoredWork, WorkPlan } from '@/lib/api/types'
import { unplannedWorks } from '@/lib/unplanned'
import music from '@/test/fixtures/profiles/music.json'

/*
 * The calendar pane's list of works with nothing planned, read against the
 * music profile the app ships: a song goes out only as its audio, clip or
 * shorts, and an experiment as nothing at all.
 */
const studio = music.config as unknown as ProfileConfig

const work = (over: Partial<ScoredWork>): ScoredWork => ({
  work_id: 'id',
  title: 'Subject',
  kind: 'song',
  status: 'draft',
  total: null,
  tier: null,
  scored_at: null,
  stale: false,
  released: 0,
  scheduled: 0,
  updated_at: '2026-11-01T10:00:00Z',
  created_at: '2026-11-01T10:00:00Z',
  collection_id: null,
  tags: [],
  marks: [],
  tier_pinned: false,
  version_count: 0,
  stage: 0,
  bookmarked_at: null,
  publications: [],
  ...over,
})

const waiting = (workId: string) => ({ work_id: workId }) as ScheduledRelease
const plan = (workId: string): WorkPlan => ({
  id: `plan-${workId}`,
  work_id: workId,
  scheduled_at: '2026-12-01',
  note: null,
  created_at: '2026-11-01T10:00:00Z',
  updated_at: '2026-11-01T10:00:00Z',
})

const ids = (rows: ScoredWork[]) => rows.map((row) => row.work_id)

describe('the works with nothing planned', () => {
  it('lists a song nothing has been made from', () => {
    const rows = [work({ work_id: 'song' })]
    expect(ids(unplannedWorks(rows, [], [], studio))).toEqual(['song'])
  })

  it('leaves out a song whose audio waits in the queue, is booked or is out', () => {
    for (const audio of [
      work({ work_id: 'audio', kind: 'audio' }),
      work({ work_id: 'audio', kind: 'audio', scheduled: 1 }),
      work({ work_id: 'audio', kind: 'audio', released: 1 }),
    ]) {
      const song = work({ work_id: 'song', publications: ['audio'] })
      const queue = audio.released + audio.scheduled === 0 ? [waiting('audio')] : []
      expect(ids(unplannedWorks([song, audio], queue, [], studio))).toEqual([])
    }
  })

  it('still lists a song whose only publications out are shorts', () => {
    // A short never opens a song (ADR 0067): the song has not gone out.
    const song = work({ work_id: 'song', publications: ['short'], released: 1 })
    const short = work({ work_id: 'short', kind: 'short', released: 1 })
    expect(ids(unplannedWorks([song, short], [], [], studio))).toEqual(['song'])
  })

  it('leaves out a song already planned on a day', () => {
    const rows = [work({ work_id: 'song' })]
    expect(ids(unplannedWorks(rows, [], [plan('song')], studio))).toEqual([])
  })

  it('leaves out an experiment: it goes out as nothing', () => {
    const rows = [work({ work_id: 'lab', kind: 'experiment' })]
    expect(ids(unplannedWorks(rows, [], [], studio))).toEqual([])
  })

  it('lists a publication with no release and leaves out one waiting in the queue', () => {
    const rows = [
      work({ work_id: 'clip', kind: 'video' }),
      work({ work_id: 'short', kind: 'short' }),
    ]
    expect(ids(unplannedWorks(rows, [waiting('short')], [], studio))).toEqual(['clip'])
  })

  it('puts the strongest first and the unscored last', () => {
    const rows = [
      work({ work_id: 'unscored', title: 'A' }),
      work({ work_id: 'weak', title: 'B', total: 40 }),
      work({ work_id: 'strong', title: 'C', kind: 'video', total: 80 }),
    ]
    expect(ids(unplannedWorks(rows, [], [], studio))).toEqual(['strong', 'weak', 'unscored'])
  })
})
