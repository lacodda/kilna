import { describe, expect, it } from 'vitest'
import type { Board, BoardCard, NoteKind } from '@/lib/api/types'
import {
  boardOfTask,
  boardTaskKey,
  chipsOf,
  counts,
  dropOf,
  entryHeadline,
  headlineOf,
  isLink,
  kindsOf,
  quoted,
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
    note_id: null,
    entry: null,
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

/** The research board, as the profile names it: works of culture linked to
 *  the library, and concepts that link to nothing. */
const research: Board = {
  key: 'research',
  label: 'Research',
  rule: 'all',
  cards: [
    { key: 'work', label: 'Work', library: true, fields: [{ key: 'why', label: 'Why' }] },
    { key: 'concept', label: 'Concept', fields: [{ key: 'term', label: 'Concept' }] },
  ],
}

const library: NoteKind = {
  key: 'source',
  label: 'Library',
  library: true,
  media: [
    { key: 'film', label: 'Film' },
    { key: 'music', label: 'Music', quoted: false },
  ],
}

const entry = (title: string, medium: string | null) => ({
  id: title,
  title,
  author: 'An Author',
  year: '2006',
  medium,
  aliases: [],
})

describe('a board of research', () => {
  it('calls a card of the library by its entry, not its first field', () => {
    const linked = card('a', '', 1, { kind: 'work', entry: entry('Old Joy', 'film') })
    expect(headlineOf(research, linked)).toBe('An Author, «Old Joy», 2006')
    expect(entryHeadline({ title: 'The Odyssey', author: null, year: null })).toBe('«The Odyssey»')
  })

  it('reads by kind: a work of culture under its medium, any other card under its kind', () => {
    const cards = [
      card('a', 'x', 1, { kind: 'work', entry: entry('Old Joy', 'film') }),
      card('b', 'x', 2, { kind: 'concept', fields: { term: 'Pharisee' } }),
      card('c', 'y', 3, { kind: 'work', entry: entry('Frances Ha', 'film') }),
      card('d', 'y', 4, { kind: 'work', entry: entry('A Song', 'music') }),
    ]
    const groups = kindsOf(research, library, cards, (label) =>
      typeof label === 'string' ? label : (label.en ?? ''),
    )
    expect(groups.map((group) => [group.name, group.cards.map((c) => c.id)])).toEqual([
      ['Film', ['a', 'c']],
      ['Concept', ['b']],
      ['Music', ['d']],
    ])
  })

  it('parts chips on lines and semicolons, once each', () => {
    expect(chipsOf('херувим; Арзамас\n  серафим.\nарзамас;;')).toEqual([
      'херувим',
      'Арзамас',
      'серафим',
    ])
  })

  it('takes a proof only as a link, and quotes no song', () => {
    expect(isLink('https://example.org/x')).toBe(true)
    expect(isLink('checked')).toBe(false)
    expect(quoted(library, entry('A Song', 'music'))).toBe(false)
    expect(quoted(library, entry('Old Joy', 'film'))).toBe(true)
    expect(quoted(library, null)).toBe(true)
  })
})
