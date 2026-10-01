import { describe, expect, it } from 'vitest'
import type { CoverBoard, IdeaCard, ProfileConfig, SiblingCover } from '@/lib/api/types'
import {
  boardOfTask,
  cardsOf,
  counts,
  coverActionOf,
  coverTaskKey,
  ideasOnMake,
  requestOf,
  shortlist,
  shows,
  totalOf,
} from '@/lib/ideas'

const look = { scheme: null, layout: null, style: null, background: null, mark: null, hero: null }

function idea(
  id: string,
  source: IdeaCard['idea']['source'],
  verdict: IdeaCard['idea']['verdict'] = null,
): IdeaCard {
  return {
    idea: {
      id,
      profile_id: 'p',
      work_id: 'w',
      source,
      from_work_id: null,
      angle: '',
      headline: id,
      concept: {} as IdeaCard['idea']['concept'],
      verdict,
      created_at: '2026-10-01T00:00:00Z',
      updated_at: '2026-10-01T00:00:00Z',
    },
    look,
    from_title: null,
  }
}

const sibling: SiblingCover = {
  work_id: 'clip',
  title: 'Harbour lights — clip',
  kind: 'video',
  concept: {} as SiblingCover['concept'],
  look,
  picture: null,
}

const board: CoverBoard = {
  format: '16:9',
  ideas: [
    idea('own', 'own'),
    idea('refined', 'refined', 'star'),
    idea('ai-1', 'ai', 'rejected'),
    idea('ai-2', 'ai', 'star'),
    idea('copied', 'sibling'),
  ],
  siblings: [sibling],
}

describe('the board of ideas', () => {
  it('lists the ideas newest first and the neighbours after them', () => {
    const cards = cardsOf(board)
    expect(
      cards.map((card) => (card.kind === 'idea' ? card.card.idea.id : card.sibling.work_id)),
    ).toEqual(['copied', 'ai-2', 'ai-1', 'refined', 'own', 'clip'])
  })

  it('counts what each filter shows', () => {
    expect(counts(cardsOf(board))).toEqual({ all: 6, star: 2, mine: 2, sibling: 2, rejected: 1 })
  })

  it('keeps a turned-down idea under "all" and off the shortlist', () => {
    const rejected = cardsOf(board).find(
      (card) => card.kind === 'idea' && card.card.idea.id === 'ai-1',
    )!
    expect(shows(rejected, 'all')).toBe(true)
    expect(shows(rejected, 'star')).toBe(false)
    expect(shortlist(board).map((card) => card.idea.id)).toEqual(['refined', 'ai-2'])
  })
})

describe('a task about a board', () => {
  it('is keyed the way the backend keys it, and read back', () => {
    const key = coverTaskKey('cover-ideas', 'w-1')
    expect(key).toBe('cover-ideas:cover:w-1')
    expect(boardOfTask(key)).toBe('w-1')
    expect(boardOfTask('release-meta:release:r-1')).toBeNull()
    expect(boardOfTask('critique:w-1')).toBeNull()
    expect(boardOfTask(undefined)).toBeNull()
  })

  it('asks for the person’s words only when they wrote any and want them worked out', () => {
    expect(requestOf(3, '  the sea in a room ', true)).toEqual({
      count: 3,
      refine: 'the sea in a room',
      more: false,
    })
    expect(requestOf(3, 'the sea', false).refine).toBeNull()
    expect(requestOf(0, '   ', true).refine).toBeNull()
    expect(totalOf(requestOf(2, 'x', true))).toBe(3)
    expect(totalOf(requestOf(0, '', true))).toBe(0)
  })
})

describe('the profile', () => {
  const config = {
    prompts: [
      { key: 'release-meta', scope: 'release', produces: 'release', kinds: ['audio'] },
      { key: 'cover-ideas', scope: 'cover', produces: 'cover-ideas', kinds: ['audio', 'short'] },
    ],
  } as unknown as ProfileConfig

  it('finds the action that proposes ideas for a kind', () => {
    expect(coverActionOf(config, 'audio')?.key).toBe('cover-ideas')
    expect(coverActionOf(config, 'song')).toBeUndefined()
  })

  it('asks for three ideas on "Make…" unless it says otherwise, and never more than five', () => {
    expect(ideasOnMake(config)).toBe(3)
    expect(ideasOnMake({ ...config, cover_ideas: 0 })).toBe(0)
    expect(ideasOnMake({ ...config, cover_ideas: 9 })).toBe(5)
  })
})
