import { describe, expect, it } from 'vitest'
import type { TimeOff } from '@/lib/api/types'
import { agendaRuns } from './agenda'

const away: TimeOff = {
  id: 'away',
  profile_id: 'p',
  starts_on: '2026-10-04',
  ends_on: '2026-10-05',
  label: 'Away',
  created_at: '',
  updated_at: '',
}

const days = ['2026-10-01', '2026-10-02', '2026-10-03', '2026-10-04', '2026-10-05', '2026-10-06']

describe('agenda runs', () => {
  it('gives a day with something on it a group, and folds the rest into gaps', () => {
    const closed = new Map([
      ['2026-10-04', away],
      ['2026-10-05', away],
    ])
    const runs = agendaRuns(days, (date) => date === '2026-10-01', closed, '2026-09-01')

    expect(runs).toEqual([
      { kind: 'day', date: '2026-10-01', closed: null },
      { kind: 'gap', from: '2026-10-02', to: '2026-10-03', days: 2, closed: null },
      // A closed span is a gap of its own: not the same kind of empty.
      { kind: 'gap', from: '2026-10-04', to: '2026-10-05', days: 2, closed: away },
      { kind: 'gap', from: '2026-10-06', to: '2026-10-06', days: 1, closed: null },
    ])
  })

  it('keeps today on its own line, empty or not', () => {
    const runs = agendaRuns(days.slice(0, 3), () => false, new Map(), '2026-10-02')
    expect(runs.map((run) => run.kind)).toEqual(['gap', 'day', 'gap'])
  })
})
