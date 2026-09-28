import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, screen, within } from '@testing-library/react'
import type { Message, Run } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The assistant as one surface (v0.81): the card's tab and the drawer from the
 * window's bar draw the same chats and the same conversation, a proposal is
 * answered from its card - applied or turned down - and what can be done with
 * an answer is in its menu rather than a row of buttons under it.
 */

const CHAT = 'chat-lanterns'
const ASKED: Message = {
  id: 'm-ask',
  chat_id: CHAT,
  role: 'user',
  body: 'Tighten the chorus',
  meta: {},
  created_at: NOW,
}

let backend: Backend

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  backend = mockBackend(answersFor(studio()))
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

/** The chat of the studio, answered with `answer` and whatever it carries. */
function transcriptWith(answer: Message) {
  return () => ({
    chat: {
      id: CHAT,
      profile_id: IDS.profile,
      work_id: IDS.song,
      title: 'Tighten the chorus',
      session_id: null,
      created_at: NOW,
      updated_at: NOW,
    },
    messages: [ASKED, answer],
  })
}

async function openTab() {
  const { client } = renderApp(`/works/${IDS.song}/assistant`)
  await settled(client)
  await screen.findByText('Tighten the chorus', { selector: 'p' })
  return client
}

describe('a proposal', () => {
  it('is turned down from its card, and the card says so instead of offering it again', async () => {
    let dismissed: string | null = null
    backend.answer('get_transcript', () =>
      transcriptWith({
        id: 'm-score',
        chat_id: CHAT,
        role: 'assistant',
        body: 'The hook holds; the verse sags.',
        meta: {
          proposal: { kind: 'score', axes: { hook: 8, lyrics: 6 } },
          ...(dismissed === null ? {} : { dismissed }),
        },
        created_at: NOW,
      })(),
    )
    backend.answer('dismiss_proposal', () => {
      dismissed = NOW
    })
    const client = await openTab()

    const apply = await screen.findByRole('button', { name: en.assistant.scoreApply })
    const card = apply.closest('section')!
    // The total the score screen would show for these marks, from the
    // profile's weights: (0.8 * 2 + 0.6 * 1.5) / 3.5.
    expect(within(card).getByText('71.4')).toBeInTheDocument()

    await act(async () =>
      fireEvent.click(within(card).getByRole('button', { name: en.assistant.dismiss })),
    )
    await settled(client)

    expect(backend.argsOf('dismiss_proposal')).toEqual([{ messageId: 'm-score' }])
    expect(await screen.findByText(en.assistant.dismissedMark)).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: en.assistant.scoreApply })).toBeNull()
    expect(backend.unanswered).toEqual([])
  })
})

describe('an answer', () => {
  it('holds what can be done with it in its menu, not in a row of buttons', async () => {
    await openTab()

    const answer = (await screen.findByText('Cut the second line; the hook lands sooner.')).closest(
      'li',
    )!
    expect(within(answer).queryByRole('button', { name: en.assistant.insert })).toBeNull()
    expect(within(answer).queryByRole('button', { name: en.assistant.keepAsNote })).toBeNull()

    fireEvent.click(within(answer).getByRole('button', { name: en.assistant.answerMenu }))
    const menu = await screen.findByRole('menu')
    expect(
      within(menu)
        .getAllByRole('menuitem')
        .map((item) => item.textContent),
    ).toEqual([en.assistant.insert, en.assistant.keepAsNote, en.assistant.copy])
  })
})

describe('the composer', () => {
  it("fills the box from the profile's actions rather than sending", async () => {
    backend.answer('render_prompt', () => 'What does the chorus promise?')
    const client = await openTab()

    fireEvent.click(screen.getByRole('button', { name: en.assistant.paletteLabel }))
    const menu = await screen.findByRole('menu')
    await act(async () => fireEvent.click(within(menu).getAllByRole('menuitem')[0]!))
    await settled(client)

    expect(screen.getByRole('textbox', { name: en.assistant.placeholder })).toHaveValue(
      'What does the chorus promise?',
    )
    expect(backend.argsOf('start_run')).toEqual([])
  })

  it('stops the run in flight from beside the send', async () => {
    const going: Run = {
      id: 'run-1',
      chat_id: CHAT,
      prompt: 'Tighten the chorus',
      state: 'running',
      events: [{ kind: 'tool', name: 'Read', detail: 'the lyric' }],
      started_at: NOW,
    }
    backend.answer('list_runs', () => [going])
    backend.answer('cancel_run', () => null)
    const client = await openTab()

    // What the run did on the way, as a line of its own.
    expect(screen.getByText('Read · the lyric')).toBeInTheDocument()
    await act(async () =>
      fireEvent.click(screen.getByRole('button', { name: en.assistant.cancel })),
    )
    await settled(client)

    expect(backend.argsOf('cancel_run')).toEqual([{ id: 'run-1' }])
  })
})

describe('without the CLI', () => {
  it('still reads the chats there are, and says why nothing can be asked', async () => {
    backend.answer('assistant_status', () => ({
      available: false,
      version: null,
      reason: 'claude was not found on PATH',
    }))
    await openTab()

    expect(screen.getByText('claude was not found on PATH')).toBeInTheDocument()
    expect(
      await screen.findByText('Cut the second line; the hook lands sooner.'),
    ).toBeInTheDocument()
  })

  it('says only that, on a work with no chats to read', async () => {
    backend.answer('assistant_status', () => ({
      available: false,
      version: null,
      reason: 'claude was not found on PATH',
    }))
    backend.answer('list_chat_summaries', () => [])
    const { client } = renderApp(`/works/${IDS.song}/assistant`)
    await settled(client)

    expect(await screen.findByText(en.assistant.unavailable)).toBeInTheDocument()
    expect(screen.queryByRole('textbox', { name: en.assistant.placeholder })).toBeNull()
  })
})

describe('the drawer', () => {
  it('lists every chat beside the open one, and leads to the work a chat is about', async () => {
    const { client } = renderApp('/dashboard')
    await settled(client)

    fireEvent.click(await screen.findByRole('button', { name: en.assistant.open }))
    const drawer = await screen.findByRole('dialog')
    await settled(client)

    // The latest chat is open beside the list, not behind a back arrow.
    const row = await within(drawer).findByRole('button', { name: /Tighten the chorus/ })
    expect(row).toHaveAttribute('aria-current', 'true')
    expect(
      await within(drawer).findByText('Cut the second line; the hook lands sooner.'),
    ).toBeInTheDocument()

    fireEvent.click(within(drawer).getByRole('button', { name: en.assistant.chatMenu }))
    const menu = await screen.findByRole('menu')
    await act(async () =>
      fireEvent.click(within(menu).getByRole('menuitem', { name: en.assistant.openWork })),
    )
    await settled(client)

    // The same conversation, now on the card's tab, with the drawer gone.
    expect(screen.queryByRole('dialog')).toBeNull()
    expect(
      await screen.findByRole('button', { name: /Tighten the chorus/, pressed: true }),
    ).toBeInTheDocument()
    expect(backend.unanswered).toEqual([])
  })
})
