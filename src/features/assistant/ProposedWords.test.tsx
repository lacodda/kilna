import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, screen, within } from '@testing-library/react'
import type { Applied, Message, WordsPackage } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * Words proposed in a chat (v0.90, ADR 0052) - by an agent's `propose_words`
 * or the Meanings action: the card reads each word with what keeping it would
 * write, a box beside each, and passes only the ticked ones to the one apply
 * every proposal goes through. Once kept, the mark says how many words the
 * record took.
 */

const CHAT = 'chat-lanterns'
const MESSAGE = 'm-words'

const PACKAGE: WordsPackage = {
  words: [
    {
      word: 'пульсар',
      bank: true,
      block: 'space',
      sung: [{ written: 'пульсар', sung: 'пульсАр' }],
    },
    {
      word: 'a lighthouse nobody keeps',
      kind: 'image',
      strictness: 'limit',
      works: [{ id: IDS.song, title: 'Paper Lanterns' }],
      note: 'the lamp unlit / the keeper gone',
    },
    { word: 'маяк', bank: true },
  ],
  dropped: [{ key: 'refusal.words.noLetters', params: { word: '—' } }],
}

let backend: Backend

function answer(meta: Record<string, unknown>): Message {
  return {
    id: MESSAGE,
    chat_id: CHAT,
    role: 'assistant',
    body: 'Three words for the record.',
    meta: { proposal: { kind: 'words', package: PACKAGE }, ...meta },
    created_at: NOW,
  }
}

function serve(meta: Record<string, unknown> = {}) {
  backend = mockBackend({
    ...answersFor(studio()),
    get_transcript: () => ({
      chat: {
        id: CHAT,
        profile_id: IDS.profile,
        work_id: IDS.song,
        title: 'Tighten the chorus',
        session_id: null,
        created_at: NOW,
        updated_at: NOW,
      },
      messages: [
        {
          id: 'm-ask',
          chat_id: CHAT,
          role: 'user',
          body: 'Tighten the chorus',
          meta: {},
          created_at: NOW,
        },
        answer(meta),
      ],
    }),
    apply_proposal: ({ messageId }) =>
      ({ message_id: messageId as string, at: NOW, terms: ['t-1', 't-2'] }) as Applied,
  })
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

describe('words proposed in a chat', () => {
  it('reads each word with what keeping it writes, and keeps only the ticked ones', async () => {
    serve()
    const { client } = renderApp(`/works/${IDS.song}/assistant`)
    await settled(client)

    const take = await screen.findByRole('button', { name: en.words.keepAll })
    await settled(client)
    const card = take.closest('section')!
    expect(within(card).getByText(en.assistant.proposedWords)).toBeInTheDocument()
    // The block named is not in the bank yet: keeping it makes one.
    expect(within(card).getByText(/a new block “space”/)).toBeInTheDocument()
    expect(within(card).getByText('пульсАр')).toBeInTheDocument()
    expect(
      within(card).getByText('to the register: Limited · works: “Paper Lanterns”'),
    ).toBeInTheDocument()
    // What could not be read is said, not dropped.
    expect(within(card).getByText('“—” has no letters.')).toBeInTheDocument()

    fireEvent.click(within(card).getByRole('checkbox', { name: 'Keep “пульсар”' }))
    await act(async () => fireEvent.click(within(card).getByRole('button', { name: 'Keep 2' })))
    await settled(client)

    expect(backend.argsOf('apply_proposal')).toEqual([
      { messageId: MESSAGE, overrides: { items: ['word:1', 'word:2'] } },
    ])
    expect(backend.unanswered).toEqual([])
  })

  it('applies the whole package when every box is ticked', async () => {
    serve()
    const { client } = renderApp(`/works/${IDS.song}/assistant`)
    await settled(client)

    await act(async () =>
      fireEvent.click(await screen.findByRole('button', { name: en.words.keepAll })),
    )
    await settled(client)

    expect(backend.argsOf('apply_proposal')).toEqual([{ messageId: MESSAGE, overrides: null }])
  })

  it('says how many words the record took once kept', async () => {
    serve({ applied: { message_id: MESSAGE, at: NOW, terms: ['t-1', 't-2'] } })
    const { client } = renderApp(`/works/${IDS.song}/assistant`)
    await settled(client)

    expect(await screen.findByText('2 words kept.')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: en.words.openBank })).toBeInTheDocument()
    expect(screen.queryByRole('checkbox')).toBeNull()
  })
})
