import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, screen, within } from '@testing-library/react'
import type { Applied, PendingProposal } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * A style's description, from the bell to the brick.
 *
 * Describing a style from its references ran and answered, and the answer
 * could only be copied by hand into the brick: the command that kept it had
 * no button (the owner's wish 3034). Since v0.77 the answer is a proposal
 * like a drafted reply, and this walks the way a person takes: the bell says
 * a description was written, the line opens the chat - which is about no
 * work, so it opens in the drawer - and the button under the answer keeps it.
 */

const CHAT = 'chat-dusk'
const MESSAGE = 'm-description'
const TEXT = 'Low sun, long reflections, teal against amber.'

let backend: Backend

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  const waiting: PendingProposal[] = [
    {
      message_id: MESSAGE,
      chat_id: CHAT,
      chat_title: 'Dusk over water',
      work_id: null,
      kind: 'description',
      created_at: NOW,
    },
  ]
  backend = mockBackend({
    ...answersFor(studio()),
    pending_proposals: () => waiting,
    // The drawer lists the chats about nothing; the style's is one of them.
    list_chat_summaries: ({ workId }) =>
      workId === undefined
        ? [{ id: CHAT, work_id: null, title: 'Dusk over water', cost_usd: 0.01, updated_at: NOW }]
        : [],
    get_transcript: ({ chatId }) =>
      chatId !== CHAT
        ? null
        : {
            chat: {
              id: CHAT,
              profile_id: IDS.profile,
              work_id: null,
              title: 'Dusk over water',
              session_id: null,
              created_at: NOW,
              updated_at: NOW,
            },
            messages: [
              {
                id: 'm-ask',
                chat_id: CHAT,
                role: 'user',
                body: 'Write the description for this style brick.',
                meta: {},
                created_at: NOW,
              },
              {
                id: MESSAGE,
                chat_id: CHAT,
                role: 'assistant',
                body: TEXT,
                meta: { proposal: { kind: 'description', style_id: IDS.brick } },
                created_at: NOW,
              },
            ],
          },
    apply_proposal: ({ messageId }) =>
      ({ message_id: messageId as string, at: NOW, style_brick: IDS.brick }) satisfies Applied,
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

describe("a style's description", () => {
  it('is kept onto its brick from the answer the bell leads to', async () => {
    const { client } = renderApp('/styles')
    await settled(client)

    fireEvent.click(await screen.findByRole('button', { name: en.journal.open }))
    const line = await screen.findByText(en.assistant.proposed.description)
    await act(async () => fireEvent.click(line))
    await settled(client)

    const keep = await screen.findByRole('button', { name: en.assistant.keepDescription })
    const answer = keep.closest('li')!
    expect(within(answer).getByText(TEXT)).toBeInTheDocument()
    // Its own button, not the note's or the version's: a second copy of the
    // text would be a second truth about the brick. Since v0.81 those live
    // in the answer's menu, which offers copying and nothing else here.
    fireEvent.click(within(answer).getByRole('button', { name: en.assistant.answerMenu }))
    expect(await screen.findByRole('menuitem', { name: en.assistant.copy })).toBeInTheDocument()
    expect(screen.queryByRole('menuitem', { name: en.assistant.keepAsNote })).toBeNull()
    expect(screen.queryByRole('menuitem', { name: en.assistant.insert })).toBeNull()
    fireEvent.keyDown(screen.getByRole('menu'), { key: 'Escape' })

    await act(async () => fireEvent.click(keep))
    await settled(client)

    expect(backend.argsOf('apply_proposal').map((args) => args.messageId)).toEqual([MESSAGE])
    expect(backend.unanswered).toEqual([])
  })
})
