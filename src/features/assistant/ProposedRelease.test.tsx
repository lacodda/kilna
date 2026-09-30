import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, screen, within } from '@testing-library/react'
import type { Applied, Message, ReleaseFieldValue } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * A release's meta proposed in a chat (v0.86): the answer's card names each
 * field in the profile's words with the text proposed for it, and takes what
 * is left of it in one press - the backend knows which fields the run
 * already filled.
 */

const CHAT = 'chat-lanterns'
const MESSAGE = 'm-meta'

const FIELDS: ReleaseFieldValue[] = [
  { key: 'title', label: 'Title', type: 'line', value: '', has_template: true },
  { key: 'description', label: 'Description', type: 'text', value: '', has_template: true },
]

let backend: Backend

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  const answer: Message = {
    id: MESSAGE,
    chat_id: CHAT,
    role: 'assistant',
    body: 'Here is what the clip could go out under.',
    meta: {
      proposal: {
        kind: 'release',
        release_id: IDS.youtube,
        // Out of the profile's order on purpose: the card reads them in it.
        fields: {
          description: 'A lantern carried through the town by the river.',
          title: 'Paper Lanterns — the clip',
        },
        unknown: ['mood'],
      },
    },
    created_at: NOW,
  }
  backend = mockBackend({
    ...answersFor(studio()),
    release_fields: () => FIELDS,
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
        answer,
      ],
    }),
    apply_proposal: ({ messageId }) =>
      ({ message_id: messageId as string, at: NOW, releases: [IDS.youtube] }) as Applied,
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

describe('a proposed release meta', () => {
  it('names each field and takes what is left of it', async () => {
    const { client } = renderApp(`/works/${IDS.song}/assistant`)
    await settled(client)

    const take = await screen.findByRole('button', { name: en.releases.proposals.takeAll })
    // The fields' names are asked once the card is drawn.
    await settled(client)
    const card = take.closest('section')!
    expect(within(card).getByText(en.releases.proposals.inChat)).toBeInTheDocument()
    expect(
      within(card)
        .getAllByRole('term')
        .map((term) => term.textContent),
    ).toEqual(['Title', 'Description'])
    expect(within(card).getByText('Paper Lanterns — the clip')).toBeInTheDocument()
    // A key the release kind does not have is said, not dropped silently.
    expect(
      within(card).getByText(en.releases.proposals.unknown.replace('{{keys}}', 'mood')),
    ).toBeInTheDocument()

    await act(async () => fireEvent.click(take))
    await settled(client)

    expect(backend.argsOf('apply_proposal')).toEqual([{ messageId: MESSAGE, overrides: null }])
    expect(backend.unanswered).toEqual([])
  })
})
