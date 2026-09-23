import { describe, expect, it } from 'vitest'
import type { RunEmission } from '@/lib/api'
import {
  announcement,
  channelOfTask,
  commentTaskKey,
  isScreenshotTask,
  movesTaskList,
  taskKey,
} from '@/lib/tasks'

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

  it('separates a scene from the work, and a block from its scene', () => {
    // The shape the backend builds, segment for segment: the frontend draws
    // the busy button from it and the backend refuses a second run by it, so
    // a difference between the two is a button that lies.
    expect(taskKey('prompts', 'w1', 's1')).toBe('prompts:w1:s1')
    expect(taskKey('prompts', 'w1', 's1', 'still')).toBe('prompts:w1:s1:still')

    // Two blocks of one scene are two tasks; the same block twice is one.
    expect(taskKey('prompts', 'w1', 's1', 'still')).not.toBe(
      taskKey('prompts', 'w1', 's1', 'motion'),
    )
    expect(taskKey('prompts', 'w1', 's1', 'still')).toBe(taskKey('prompts', 'w1', 's1', 'still'))

    // And a block task is not the whole-scene task.
    expect(taskKey('prompts', 'w1', 's1', 'still')).not.toBe(taskKey('prompts', 'w1', 's1'))
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

describe('comment task keys', () => {
  it('names a reply task by its comment, the way the backend does', () => {
    expect(commentTaskKey('reply-to-comment', 'c1')).toBe('reply-to-comment:comment:c1')
  })

  it('tells a screenshot being read from any other task of the action', () => {
    expect(isScreenshotTask('read-comment:channel:main:shot', 'read-comment')).toBe(true)
    expect(isScreenshotTask('read-comment:comment:c1', 'read-comment')).toBe(false)
    expect(isScreenshotTask('critique:channel:x', 'read-comment')).toBe(false)
  })
})

describe('channelOfTask', () => {
  it('reads the channel back out of a screenshot key, escapes undone', () => {
    expect(channelOfTask('read-comment:channel:live%3A 100%25:shot')).toBe('live: 100%')
    expect(channelOfTask('critique:work-1')).toBeUndefined()
  })
})
