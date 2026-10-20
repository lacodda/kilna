import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, waitFor, within } from '@testing-library/react'
import type { BoardCard, BoardView } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio, type Studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The Conception tab (v0.99, ADR 0065): what the window asks the backend for
 * when a person starts a brief, puts a card on a board, asks for more, judges
 * and writes a card, digs where the last answer pointed, copies the order out
 * and asks for the text to be written from it. What lands is the backend's;
 * these hold the window to what it sends.
 */

let backend: Backend
let workspace: Studio

function card(id: string, line: string, over: Partial<BoardCard> = {}): BoardCard {
  return {
    id,
    profile_id: IDS.profile,
    work_id: IDS.song,
    board: 'scenarios',
    kind: 'scenario',
    topic: '',
    position: 1,
    source: 'ai',
    verdict: null,
    fields: { line },
    created_at: '2026-10-19T10:00:00Z',
    updated_at: '2026-10-19T10:00:00Z',
    ...over,
  }
}

function boards(cards: BoardCard[], directions: string[] = []): BoardView[] {
  return [{ board: 'scenarios', cards, directions }]
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  workspace = studio()
  backend = mockBackend({
    ...answersFor(workspace),
    add_own_card: ({ workId, words }) =>
      card('own', words as string, { work_id: workId as string, source: 'own' }),
    judge_card: ({ id, verdict }) => ({ ...card(id as string, 'x'), verdict }),
    update_card: ({ id }) => card(id as string, 'x'),
    start_board_task: ({ id, action, board }) => ({
      chatId: 'chat-cards',
      runId: 'run-cards',
      taskKey: `${action as string}:board:${id as string}:-:${board as string}`,
      title: 'Scenarios',
    }),
    start_task: ({ workId, action }) => ({
      chatId: 'chat-write',
      runId: 'run-write',
      taskKey: `${action as string}:${workId as string}`,
      title: 'Write from the brief',
    }),
    create_version: ({ workId, version: made }) => ({
      id: 'brief-1',
      work_id: workId,
      ...(made as object),
      revision: 1,
      label: null,
      meta: {},
      created_at: '2026-10-19T10:00:00Z',
      parent_version_id: null,
      trial_id: null,
    }),
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

async function openTab(workId: string = IDS.song) {
  const { client } = renderApp(`/works/${workId}/conception`)
  await settled(client)
}

describe('the Conception tab', () => {
  it('starts the brief from its skeleton, never as the work’s current text', async () => {
    await openTab()
    fireEvent.click(await screen.findByRole('button', { name: /Start the brief/ }))
    await waitFor(() => expect(backend.argsOf('create_version')).toHaveLength(1))
    const made = backend.argsOf('create_version')[0]!
    expect(made.workId).toBe(IDS.song)
    expect(made.version).toMatchObject({ role: 'brief', make_current: false })
    expect((made.version as { body: string }).body).toContain('## What it is about')
  })

  it('puts the person’s card on the board first, then asks for more of the assistant’s', async () => {
    await openTab()
    const words = await screen.findByRole('textbox', { name: en.board.own })
    fireEvent.change(words, { target: { value: 'A keeper counts the ships' } })
    fireEvent.change(screen.getByRole('textbox', { name: en.board.direction }), {
      target: { value: 'old maps' },
    })
    fireEvent.click(screen.getByRole('button', { name: en.board.generate }))

    await waitFor(() => expect(backend.argsOf('start_board_task')).toHaveLength(1))
    expect(backend.argsOf('add_own_card')).toEqual([
      { workId: IDS.song, board: 'scenarios', kind: null, words: 'A keeper counts the ships' },
    ])
    expect(backend.argsOf('start_board_task')[0]).toEqual({
      id: IDS.song,
      action: 'scenarios',
      board: 'scenarios',
      request: {
        count: 4,
        refine: 'A keeper counts the ships',
        direction: 'old maps',
        more: false,
      },
    })
  })

  it('stars a card, turns another down, and writes a field in place', async () => {
    backend.answer('work_boards', () =>
      boards([card('one', 'A keeper counts the ships'), card('two', 'A storm', { position: 2 })]),
    )
    await openTab()

    const one = await screen.findByRole('article', { name: 'A keeper counts the ships' })
    fireEvent.click(within(one).getByRole('button', { name: en.board.star }))
    await waitFor(() =>
      expect(backend.argsOf('judge_card')).toEqual([{ id: 'one', verdict: 'star' }]),
    )
    const two = screen.getByRole('article', { name: 'A storm' })
    fireEvent.click(within(two).getByRole('button', { name: en.board.reject }))
    await waitFor(() => expect(backend.argsOf('judge_card')).toHaveLength(2))
    expect(backend.argsOf('judge_card')[1]).toEqual({ id: 'two', verdict: 'rejected' })

    const image = within(one).getByRole('textbox', { name: 'The image through it' })
    fireEvent.focus(image)
    fireEvent.change(image, { target: { value: 'the lamp' } })
    fireEvent.blur(image)
    await waitFor(() =>
      expect(backend.argsOf('update_card')).toEqual([
        { id: 'one', patch: { fields: { image: 'the lamp' } } },
      ]),
    )
  })

  it('digs where the last answer pointed with one click', async () => {
    backend.answer('work_boards', () =>
      boards([card('one', 'A keeper counts the ships')], ['the sea as a border']),
    )
    await openTab()
    fireEvent.click(await screen.findByRole('button', { name: 'the sea as a border' }))
    await waitFor(() => expect(backend.argsOf('start_board_task')).toHaveLength(1))
    expect(backend.argsOf('start_board_task')[0]?.request).toEqual({
      count: 4,
      refine: null,
      direction: 'the sea as a border',
      more: false,
    })
  })

  it('copies the order out and asks for the text to be written from it', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined)
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true })
    backend.answer('conception_order', () => '# “Paper Lanterns”\n\nthe order')
    await openTab()

    fireEvent.click(await screen.findByRole('button', { name: en.conception.copy }))
    await waitFor(() => expect(writeText).toHaveBeenCalledWith('# “Paper Lanterns”\n\nthe order'))

    fireEvent.click(screen.getByRole('button', { name: 'Write from the brief' }))
    await waitFor(() =>
      expect(backend.argsOf('start_task')).toEqual([
        expect.objectContaining({ workId: IDS.song, action: 'write' }),
      ]),
    )
  })

  it('holds only what is attached on a work of a kind prepared on no conception', async () => {
    await openTab(IDS.video)
    expect(
      await screen.findByText('An old man who sells lanterns by the bridge.'),
    ).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /Start the/ })).not.toBeInTheDocument()
    expect(screen.queryByRole('textbox', { name: en.board.own })).not.toBeInTheDocument()
  })
})
