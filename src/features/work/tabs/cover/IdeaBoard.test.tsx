import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, waitFor, within } from '@testing-library/react'
import type { CoverBoard, IdeaCard, SiblingCover, WorkPatch } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, coverOf, IDS, NOW, studio, type Studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The board of ideas on the Cover tab (v0.89): what the window asks the
 * backend for when a person writes an idea, asks for more, judges a card or
 * takes one into the constructor. What lands on the board is the backend's;
 * these hold the window to what it sends.
 */

let backend: Backend
let workspace: Studio

const LOOK = { scheme: null, layout: null, style: null, background: null, mark: null, hero: null }

function idea(id: string, over: Partial<IdeaCard['idea']> = {}): IdeaCard {
  return {
    idea: {
      id,
      profile_id: IDS.profile,
      work_id: IDS.video,
      source: 'ai',
      from_work_id: null,
      angle: 'from below',
      headline: `Idea ${id}`,
      concept: coverOf({ idea: `What ${id} says`, scene: `what ${id} shows` }),
      verdict: null,
      created_at: '2026-09-20T10:00:00Z',
      updated_at: '2026-09-20T10:00:00Z',
      ...over,
    },
    look: LOOK,
    from_title: null,
  }
}

const SIBLING: SiblingCover = {
  work_id: IDS.audio,
  title: 'Paper Lanterns — audio',
  kind: 'audio',
  concept: coverOf({ idea: 'A lantern on the water', scene: 'a lantern on dark water' }),
  look: LOOK,
  picture: null,
}

function board(over: Partial<CoverBoard> = {}): CoverBoard {
  return { format: '16:9', ideas: [], siblings: [], ...over }
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  workspace = studio()
  backend = mockBackend({
    ...answersFor(workspace),
    update_work: ({ id, patch }) => {
      const work = workspace.works.find((one) => one.id === id)!
      Object.assign(work, patch as WorkPatch)
      return work
    },
    add_own_idea: ({ workId, words }) =>
      idea('own', {
        source: 'own',
        work_id: workId as string,
        concept: coverOf({ idea: words as string }),
      }).idea,
    start_cover_task: ({ id, action }) => ({
      chatId: 'chat-ideas',
      runId: 'run-ideas',
      taskKey: `${action as string}:cover:${id as string}`,
      title: 'Cover ideas',
    }),
    judge_idea: ({ id, verdict }) => ({ ...idea(id as string).idea, verdict }),
    judge_sibling_cover: ({ siblingId }) =>
      idea('copied', { source: 'sibling', from_work_id: siblingId as string }).idea,
    take_idea: () => workspace.works.find((one) => one.id === IDS.video)!,
    take_sibling_cover: () => workspace.works.find((one) => one.id === IDS.video)!,
    stop_task: () => true,
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

async function openBoard() {
  const { client } = renderApp(`/works/${IDS.video}/cover`)
  await screen.findByRole('region', { name: en.ideas.composer })
  await settled(client)
}

describe('the board of ideas', () => {
  it('opens on a cover with nothing in it, and asking puts the person’s idea on it first', async () => {
    await openBoard()

    const words = screen.getByRole('textbox', { name: en.ideas.own })
    fireEvent.change(words, { target: { value: 'The lantern keeper asleep by the river' } })
    fireEvent.click(screen.getByRole('button', { name: en.ideas.generate }))

    await waitFor(() => expect(backend.argsOf('start_cover_task')).toHaveLength(1))
    expect(backend.argsOf('add_own_idea')).toEqual([
      { workId: IDS.video, words: 'The lantern keeper asleep by the river' },
    ])
    expect(backend.argsOf('start_cover_task')[0]).toEqual({
      id: IDS.video,
      action: 'cover-ideas',
      request: { count: 3, refine: 'The lantern keeper asleep by the river', more: false },
    })
  })

  it('stars a card, turns another down, and takes one into the constructor', async () => {
    backend.answer('cover_board', () => board({ ideas: [idea('one'), idea('two')] }))
    await openBoard()

    const one = screen.getByRole('article', { name: 'Idea one' })
    fireEvent.click(within(one).getByRole('button', { name: en.ideas.star }))
    await waitFor(() =>
      expect(backend.argsOf('judge_idea')).toEqual([{ id: 'one', verdict: 'star' }]),
    )

    const two = screen.getByRole('article', { name: 'Idea two' })
    fireEvent.click(within(two).getByRole('button', { name: en.ideas.reject }))
    await waitFor(() => expect(backend.argsOf('judge_idea')).toHaveLength(2))
    expect(backend.argsOf('judge_idea')[1]).toEqual({ id: 'two', verdict: 'rejected' })

    fireEvent.click(within(one).getByRole('button', { name: en.ideas.take }))
    await waitFor(() => expect(backend.argsOf('take_idea')).toEqual([{ id: 'one' }]))
    // Taken in, the idea is shown where the cover is built from it.
    expect(await screen.findByRole('region', { name: en.cover.idea.title })).toBeInTheDocument()
  })

  it('offers a neighbour’s cover, and a star copies it onto the board', async () => {
    backend.answer('cover_board', () => board({ siblings: [SIBLING] }))
    await openBoard()

    const offered = screen.getByRole('article', { name: SIBLING.title })
    fireEvent.click(within(offered).getByRole('button', { name: en.ideas.star }))

    await waitFor(() =>
      expect(backend.argsOf('judge_sibling_cover')).toEqual([
        { workId: IDS.video, siblingId: IDS.audio, verdict: 'star' },
      ]),
    )
  })

  it('stands skeletons while ideas are written, and Stop stops the run whoever started it', async () => {
    const key = `cover-ideas:cover:${IDS.video}`
    backend.answer('active_tasks', () => [key])
    await openBoard()

    // As many as were last asked for on this board, or as "Make…" asks for.
    const line = await screen.findByText(/Writing \d ideas/)
    const status = line.closest<HTMLElement>('[role="status"]')!
    fireEvent.click(within(status).getByRole('button', { name: en.ideas.stop }))

    await waitFor(() => expect(backend.argsOf('stop_task')).toEqual([{ key }]))
  })

  it('asks for more in the direction of the shortlist only once something is starred', async () => {
    backend.answer('cover_board', () => board({ ideas: [idea('one')] }))
    await openBoard()
    const more = screen.getByRole('button', { name: /More in this direction/ })
    expect(more).toHaveAttribute('aria-disabled', 'true')

    backend.answer('cover_board', () => board({ ideas: [idea('one', { verdict: 'star' })] }))
    fireEvent.click(
      within(screen.getByRole('article', { name: 'Idea one' })).getByRole('button', {
        name: en.ideas.star,
      }),
    )
    await waitFor(() =>
      expect(screen.getByRole('button', { name: /More in this direction/ })).not.toHaveAttribute(
        'aria-disabled',
        'true',
      ),
    )
    fireEvent.click(screen.getByRole('button', { name: /More in this direction/ }))
    await waitFor(() => expect(backend.argsOf('start_cover_task')).toHaveLength(1))
    expect(backend.argsOf('start_cover_task')[0]?.request).toEqual({
      count: 3,
      refine: null,
      more: true,
    })
  })
})
