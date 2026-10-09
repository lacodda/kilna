import { describe, expect, it } from 'vitest'
import type { Trial, TrialBoard, TrialCard } from '@/lib/api/types'
import {
  aroundOfLabTask,
  boardOfLabTask,
  counts,
  fix,
  harvestKindsOf,
  labActionOf,
  labKindsFor,
  labTaskKey,
  seriesOf,
  sweep,
} from '@/lib/trials'
import { studio } from '@/test/workspace'

function card(id: string, over: Partial<Trial> = {}): TrialCard {
  return {
    trial: {
      id,
      profile_id: 'p',
      work_id: 'lab',
      series: 'sweep',
      position: 1,
      parent_id: null,
      angle: '',
      body: id,
      bricks: [],
      reference: '',
      outcome: '',
      verdict: null,
      source_version_id: null,
      run_first: false,
      created_at: '2026-10-01T10:00:00Z',
      updated_at: '2026-10-01T10:00:00Z',
      ...over,
    },
    takes: [],
    harvest: [],
    lost_anchors: [],
    source: null,
  }
}

function board(cards: TrialCard[], series: string[]): TrialBoard {
  return {
    work_id: 'lab',
    anchors: [],
    harvest_role: 'style',
    harvest_kinds: ['song'],
    composition: 'sound',
    series,
    trials: cards,
  }
}

describe('the board as a tree', () => {
  const cards = [
    card('core', { verdict: 'drop' }),
    card('slower', { parent_id: 'core', verdict: 'keep' }),
    card('denser', { parent_id: 'core' }),
    card('elsewhere', { parent_id: 'core', series: 'around' }),
  ]

  it('nests a variation under the trial it varies, in its series', () => {
    const groups = seriesOf(board(cards, ['sweep', 'around']))
    expect(groups.map((group) => group.series)).toEqual(['sweep', 'around'])
    expect(groups[0]!.rows.map((row) => [row.card.trial.id, row.depth])).toEqual([
      ['core', 0],
      ['slower', 1],
      ['denser', 1],
    ])
    expect(groups[1]!.rows.map((row) => [row.card.trial.id, row.depth])).toEqual([['elsewhere', 0]])
  })

  it('raises the children of a trial a filter hides', () => {
    const groups = seriesOf(board(cards, ['sweep', 'around']), 'keep')
    expect(groups).toHaveLength(1)
    expect(groups[0]!.rows.map((row) => [row.card.trial.id, row.depth])).toEqual([['slower', 0]])
  })

  it('counts each filter', () => {
    expect(counts(cards)).toEqual({ all: 4, keep: 1, open: 2, drop: 1 })
  })
})

describe('a task about a board', () => {
  it('runs under the key the backend reads its answer by', () => {
    const key = labTaskKey('propose-trials', 'lab', fix('t1', 'sweep'))
    expect(key).toBe('propose-trials:lab:lab:t1:fix:sweep')
    expect(boardOfLabTask(key)).toBe('lab')
    expect(aroundOfLabTask(key)).toBe('t1')
    expect(aroundOfLabTask(labTaskKey('propose-trials', 'lab', sweep(6, 'a: b')))).toBeNull()
  })
})

describe('the lab in the profile', () => {
  const config = studio().profile.config

  it('finds the action, where trials go, and what an experiment is made of', () => {
    expect(labActionOf(config, 'experiment')?.key).toBe('propose-trials')
    expect(labActionOf(config, 'song')).toBeUndefined()
    expect(harvestKindsOf(config, 'experiment')).toEqual(['song'])
    expect(labKindsFor(config, 'song')).toEqual(['experiment'])
    expect(labKindsFor(config, 'video')).toEqual([])
  })
})
