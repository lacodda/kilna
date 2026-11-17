import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, waitFor, within } from '@testing-library/react'
import type { Applied, Message, ProposedNote, WordsPackage } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * What an agent proposes about notes and words that are already there
 * (v0.116, FR-08 8.2-8.3): a change to a note is read against what the note
 * holds, notes are kept one by one, and a word the record keeps says what
 * keeping the proposal changes in it - was, becomes - or that it is taken
 * out or split.
 */

const CHAT = 'chat-lanterns'
const MESSAGE = 'm-proposal'

const NOTES: ProposedNote[] = [
  {
    body: 'the road as a witness, and a judge',
    tags: ['idea', 'road'],
    note: 'note-concept',
    before: { title: 'Concept', body: 'the road as a witness', tags: ['idea'] },
  },
  { title: 'Bridge', body: 'try a key change', layer: 'internal' },
]

const WORDS: WordsPackage = {
  words: [
    {
      word: 'lighthouse',
      known: true,
      strictness: 'ban',
      changes: [{ field: 'strictness', was: 'rare', becomes: 'ban' }],
    },
    { word: 'tpyo', action: 'delete', known: true },
    { word: 'spacesuit, isotope', action: 'split', into: ['spacesuit', 'isotope'], known: true },
  ],
  warnings: [{ key: 'refusal.words.separator', params: { word: 'candle / candles' } }],
}

let backend: Backend

function serve(proposal: Record<string, unknown>) {
  const message: Message = {
    id: MESSAGE,
    chat_id: CHAT,
    role: 'assistant',
    body: 'A proposal.',
    meta: { proposal },
    created_at: NOW,
  }
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
        { id: 'm-ask', chat_id: CHAT, role: 'user', body: 'Ask', meta: {}, created_at: NOW },
        message,
      ],
    }),
    apply_proposal: ({ messageId }) =>
      ({ message_id: messageId as string, at: NOW, notes: ['n-1'] }) as Applied,
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

describe('notes proposed several at once', () => {
  it('shows a change against what the note holds and keeps only the ticked notes', async () => {
    serve({ kind: 'notes', notes: NOTES })
    const { client } = renderApp(`/works/${IDS.song}/assistant`)
    await settled(client)

    const take = await screen.findByRole('button', { name: en.assistant.keepNotesAll })
    const card = take.closest('section')!
    // The change: called by the note it changes, with was and becomes.
    expect(within(card).getByText('Concept')).toBeInTheDocument()
    expect(within(card).getByText(en.assistant.noteChange)).toBeInTheDocument()
    expect(within(card).getByText('the road as a witness')).toBeInTheDocument()
    expect(within(card).getByText('the road as a witness, and a judge')).toBeInTheDocument()
    expect(within(card).getByText('idea, road')).toBeInTheDocument()
    // The new one, with who may read it.
    expect(within(card).getByText('Bridge')).toBeInTheDocument()
    expect(within(card).getByText(en.assistant.noteLayer.internal)).toBeInTheDocument()

    // The new note left out: only the change is asked for, by its key.
    fireEvent.click(
      within(card).getByRole('checkbox', {
        name: en.assistant.keepNoteItem.replace('{{title}}', 'Bridge'),
      }),
    )
    fireEvent.click(
      within(card).getByRole('button', { name: en.assistant.keepNotes.replace('{{n}}', '1') }),
    )
    await waitFor(() => expect(backend.argsOf('apply_proposal')).toHaveLength(1))
    expect(backend.argsOf('apply_proposal')[0]).toMatchObject({
      messageId: MESSAGE,
      overrides: { items: ['note:0'] },
    })
  })
})

describe('words proposed for words the record keeps', () => {
  it('says what changes, what is taken out and split, and warns of a word of several', async () => {
    serve({ kind: 'words', package: WORDS })
    const { client } = renderApp(`/works/${IDS.song}/assistant`)
    await settled(client)

    const take = await screen.findByRole('button', { name: en.words.keepAll })
    const card = take.closest('section')!
    expect(within(card).getByText(en.words.changes)).toBeInTheDocument()
    expect(within(card).getByText('rare')).toBeInTheDocument()
    expect(within(card).getByText('ban')).toBeInTheDocument()
    expect(within(card).getByText(en.words.takeOut)).toBeInTheDocument()
    expect(
      within(card).getByText(en.words.splitInto.replace('{{parts}}', 'spacesuit, isotope')),
    ).toBeInTheDocument()
    expect(within(card).getByText(/“candle \/ candles” reads as several words/)).toBeInTheDocument()
  })
})
