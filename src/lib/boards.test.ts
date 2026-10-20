import { describe, expect, it } from 'vitest'
import type { BoardCard } from '@/lib/api/types'
import {
  boardOfTask,
  boardTaskKey,
  counts,
  dropOf,
  requestOf,
  topicsOf,
  totalOf,
} from '@/lib/boards'

function card(
  id: string,
  topic: string,
  position: number,
  over: Partial<BoardCard> = {},
): BoardCard {
  return {
    id,
    profile_id: 'p',
    work_id: 'w',
    board: 'scenarios',
    kind: 'scenario',
    topic,
    position,
    source: 'ai',
    verdict: null,
    fields: { line: id },
    created_at: '',
    updated_at: '',
    ...over,
  }
}

describe('a board of the conception', () => {
  it('groups its cards under topics in the order their first cards stand', () => {
    const groups = topicsOf([card('c', 'Sea', 3), card('a', 'Town', 1), card('b', 'Sea', 2)])
    expect(groups.map((group) => [group.name, group.cards.map((one) => one.id)])).toEqual([
      ['Town', ['a']],
      ['Sea', ['b', 'c']],
    ])
  })

  it('counts each filter, a turned-down card among all', () => {
    const tally = counts([
      card('a', '', 1, { verdict: 'star' }),
      card('b', '', 2, { verdict: 'rejected', source: 'own' }),
      card('c', '', 3, { source: 'refined' }),
    ])
    expect(tally).toEqual({ all: 3, star: 1, mine: 2, rejected: 1 })
  })

  it('names a task the way the backend keys it, a board with colons included', () => {
    const key = boardTaskKey('scenarios', 'w1', 'a:b', true)
    expect(key).toBe('scenarios:board:w1:own:a:b')
    expect(boardOfTask(key)).toEqual({ workId: 'w1', board: 'a:b' })
    expect(boardOfTask('cover-ideas:cover:w1')).toBeNull()
    expect(boardOfTask(undefined)).toBeNull()
  })

  it('asks for the person’s words worked out only when asked and written', () => {
    expect(requestOf(3, '  a keeper  ', true, ' maps ', false)).toEqual({
      count: 3,
      refine: 'a keeper',
      direction: 'maps',
      more: false,
    })
    expect(requestOf(3, 'a keeper', false, '', true)).toEqual({
      count: 3,
      refine: null,
      direction: null,
      more: true,
    })
    expect(totalOf(requestOf(2, 'a keeper', true, '', false))).toBe(3)
  })

  it('reads a drop off what it was let go over: before a card, or at the end of a topic', () => {
    // An element as far as `closest` and `dataset` go: the logic runs where
    // there is no document.
    const element = (found: Record<string, Record<string, string>>) =>
      ({
        closest: (selector: string) => {
          const key = Object.keys(found).find((one) => selector.includes(one))
          return key === undefined ? null : { dataset: found[key] }
        },
      }) as unknown as Element
    const onCard = element({
      'data-card-slot': { cardSlot: '2', cardTopic: 'Sea' },
      'data-topic-drop': { topicDrop: 'Sea' },
    })
    const beside = element({ 'data-topic-drop': { topicDrop: 'Sea' } })
    expect(dropOf(onCard)).toEqual({ topic: 'Sea', index: 2 })
    expect(dropOf(beside)).toEqual({ topic: 'Sea', index: Number.MAX_SAFE_INTEGER })
    expect(dropOf(element({}))).toBeNull()
    expect(dropOf(null)).toBeNull()
  })
})
