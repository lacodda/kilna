import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, screen, within } from '@testing-library/react'
import type { Applied, Message, StylesPackage } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * Styles an agent proposed for the dictionary (v0.134, `propose_style`): a
 * new one with its contract and a rewrite, each with a box; only the ticked
 * ones go to the one apply every proposal goes through.
 */

const CHAT = 'chat-lanterns'
const MESSAGE = 'm-styles'

const PACKAGE: StylesPackage = {
  styles: [
    {
      type: 'pose',
      name: 'Arms crossed',
      description: 'arms crossed, weight on one leg',
      contract: { needs: ['subject:person', 'crop:min:waist'] },
    },
    { id: IDS.brick, description: 'Low sun, teal against amber.', was: 'Dusk over water' },
  ],
}

let backend: Backend

function serve() {
  const message: Message = {
    id: MESSAGE,
    chat_id: CHAT,
    role: 'assistant',
    body: 'Two styles for the dictionary.',
    meta: { proposal: { kind: 'styles', package: PACKAGE } },
    created_at: NOW,
  }
  backend = mockBackend({
    ...answersFor(studio()),
    get_transcript: () => ({
      chat: {
        id: CHAT,
        profile_id: IDS.profile,
        work_id: IDS.song,
        title: 'Styles',
        session_id: null,
        created_at: NOW,
        updated_at: NOW,
      },
      messages: [message],
    }),
    apply_proposal: ({ messageId }) =>
      ({ message_id: messageId as string, at: NOW, style_bricks: ['b-new'] }) as Applied,
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

describe('styles proposed in a chat', () => {
  it('reads each style with its contract, and keeps only the ticked ones', async () => {
    serve()
    const { client } = renderApp(`/works/${IDS.song}/assistant`)
    await settled(client)

    const take = await screen.findByRole('button', { name: en.phrases.keepAll })
    const card = take.closest('section')!
    expect(within(card).getByText(en.assistant.proposedStyles)).toBeInTheDocument()
    expect(within(card).getByText('subject:person · crop:min:waist')).toBeInTheDocument()
    expect(within(card).getByText(en.styles.rewrite)).toBeInTheDocument()

    fireEvent.click(within(card).getByRole('checkbox', { name: 'Keep “Dusk over water”' }))
    await act(async () => fireEvent.click(within(card).getByRole('button', { name: 'Keep 1' })))
    await settled(client)
    expect(backend.argsOf('apply_proposal')).toEqual([
      { messageId: MESSAGE, overrides: { items: ['style:0'] } },
    ])
  })
})
