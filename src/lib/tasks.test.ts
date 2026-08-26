import { describe, expect, it } from 'vitest'
import type { RunEmission } from '@/lib/api'
import { announcement, movesTaskList, taskKey } from '@/lib/tasks'

const emission = (over: Partial<RunEmission> = {}): RunEmission => ({
  run_id: 'r1',
  chat_id: 'c1',
  event: { kind: 'finished', body: 'done' },
  ...over,
})

describe('taskKey', () => {
  it('names the action and the work, not the run', () => {
    expect(taskKey('critique', 'w1')).toBe(taskKey('critique', 'w1'))
    expect(taskKey('critique', 'w1')).not.toBe(taskKey('critique', 'w2'))
    expect(taskKey('critique', 'w1')).not.toBe(taskKey('score', 'w1'))
  })
})

describe('announcement', () => {
  it('announces a finished task', () => {
    expect(announcement(emission({ task: 'critique:w1' }))).toBe('done')
  })

  it('announces a task that ended without an answer', () => {
    expect(
      announcement(emission({ task: 'critique:w1', event: { kind: 'failed', message: 'no' } })),
    ).toBe('ended')
  })

  it('says nothing about a prompt someone typed', () => {
    expect(announcement(emission())).toBeNull()
  })

  it('says nothing while a task is still going', () => {
    expect(
      announcement(
        emission({ task: 'critique:w1', event: { kind: 'text', body: 'half an answer' } }),
      ),
    ).toBeNull()
    expect(
      announcement(
        emission({ task: 'critique:w1', event: { kind: 'started', session_id: 's' } }),
      ),
    ).toBeNull()
  })

  it('stays silent about a task stopped by hand', () => {
    expect(announcement(emission({ task: 'critique:w1', event: { kind: 'stopped' } }))).toBeNull()
  })
})

describe('movesTaskList', () => {
  it('counts every boundary a run can reach', () => {
    for (const event of [
      { kind: 'started', session_id: 's' },
      { kind: 'finished', body: 'done' },
      { kind: 'failed', message: 'no' },
      { kind: 'stopped' },
    ] as const) {
      expect(movesTaskList(emission({ event }))).toBe(true)
    }
  })

  it('ignores the blocks of an answer in between', () => {
    expect(movesTaskList(emission({ event: { kind: 'text', body: 'a line' } }))).toBe(false)
    expect(
      movesTaskList(emission({ event: { kind: 'tool', name: 'Read', detail: 'x' } })),
    ).toBe(false)
  })
})
