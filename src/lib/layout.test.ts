import { describe, expect, it } from 'vitest'
import type { ScheduledRelease } from './api'
import { ghostsOf } from './layout'

const queued = (id: string, title: string): ScheduledRelease =>
  ({
    id,
    work_id: `work-${id}`,
    work_title: title,
    kind: 'clip',
  }) as ScheduledRelease

describe('ghostsOf', () => {
  it('groups placements by day, named from the queue', () => {
    const ghosts = ghostsOf(
      [
        { release_id: 'r1', date: '2026-09-02' },
        { release_id: 'r2', date: '2026-09-05' },
      ],
      [queued('r1', 'First'), queued('r2', 'Second')],
    )

    expect(ghosts.get('2026-09-02')).toEqual([
      { releaseId: 'r1', workId: 'work-r1', title: 'First', kind: 'clip' },
    ])
    expect(ghosts.get('2026-09-05')?.[0]?.title).toBe('Second')
  })

  it('drops a placement whose release left the queue rather than drawing a blank', () => {
    const ghosts = ghostsOf([{ release_id: 'gone', date: '2026-09-02' }], [queued('r1', 'First')])

    expect(ghosts.size).toBe(0)
  })

  it('keeps two ghosts apart when a plan somehow names one day twice', () => {
    const ghosts = ghostsOf(
      [
        { release_id: 'r1', date: '2026-09-02' },
        { release_id: 'r2', date: '2026-09-02' },
      ],
      [queued('r1', 'First'), queued('r2', 'Second')],
    )

    expect(ghosts.get('2026-09-02')).toHaveLength(2)
  })
})
