import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, screen } from '@testing-library/react'
import type { Applied, Message, PackagedTrial } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * A package for an experiment proposed by an agent (v0.95): its card says
 * how many trials it lands on the board with, beside its versions and its
 * fields - an experiment proposed without saying so read as a bare brief.
 */

const CHAT = 'chat-lab'
const MESSAGE = 'm-lab'

const trial = (angle: string): PackagedTrial => ({
  series: 'A sweep of the field',
  angle,
  body: `${angle}, fuzz bass, galloping toms`,
  bricks: [],
  reference: '',
  outcome: '',
  run_first: false,
})

let backend: Backend

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  const answer: Message = {
    id: MESSAGE,
    chat_id: CHAT,
    role: 'assistant',
    body: 'A board for the experiment.',
    meta: {
      proposal: {
        kind: 'work',
        versions: [{ role: 'brief', body: 'Find the break under the guitars.' }],
        trials: [trial('Slower'), trial('Drier'), trial('Louder')],
      },
    },
    created_at: NOW,
  }
  // The experiment's own chat, the one an agent's proposals wait in.
  const workspace = studio()
  workspace.chats.push({
    id: CHAT,
    work_id: IDS.lab,
    work_title: 'Breaks under guitars',
    title: 'Claude Code',
    first_prompt: 'A board for the experiment',
    cost_usd: 0,
    updated_at: NOW,
  })
  backend = mockBackend({
    ...answersFor(workspace),
    get_transcript: () => ({
      chat: {
        id: CHAT,
        profile_id: IDS.profile,
        work_id: IDS.lab,
        title: 'Claude Code',
        session_id: null,
        created_at: NOW,
        updated_at: NOW,
      },
      messages: [answer],
    }),
    apply_proposal: ({ messageId }) => ({ message_id: messageId as string, at: NOW }) as Applied,
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

describe('a proposed package with trials', () => {
  it('says how many trials it lands with and applies in one press', async () => {
    const { client } = renderApp(`/works/${IDS.lab}/assistant`)
    await settled(client)

    const apply = await screen.findByRole('button', { name: en.assistant.applyPackage })
    const card = apply.closest('section')!
    expect(card.textContent).toContain(en.assistant.packageTrials_other.replace('{{count}}', '3'))

    await act(async () => fireEvent.click(apply))
    await settled(client)

    expect(backend.argsOf('apply_proposal')).toEqual([{ messageId: MESSAGE, overrides: null }])
    expect(backend.unanswered).toEqual([])
  })
})
