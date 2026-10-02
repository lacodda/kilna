import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, screen, within } from '@testing-library/react'
import { emit } from '@tauri-apps/api/event'
import type {
  Applied,
  ReleaseFieldValue,
  ReleaseProposal,
  RunEmission,
  StartedTask,
} from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The release meta on the overview's release block (v0.86): the profile's action writes
 * what a release goes out under in the background, from a button on the
 * release. The fields nobody had written are filled when the answer lands;
 * the started ones wait under the fields as a proposal, read field by field
 * against what is there and taken in part - or turned down.
 */

const ACTION = 'release-meta'
const TASK = `${ACTION}:release:${IDS.youtube}`
const MESSAGE = 'm-release-meta'

const FIELDS: ReleaseFieldValue[] = [
  { key: 'title', label: 'Title', type: 'line', value: 'Paper Lanterns', has_template: true },
  {
    key: 'description',
    label: 'Description',
    type: 'text',
    value: 'Typed by hand.',
    has_template: true,
  },
]

const PROPOSAL: ReleaseProposal = {
  message_id: MESSAGE,
  chat_id: 'chat-meta',
  client: null,
  created_at: NOW,
  fields: [
    {
      key: 'title',
      label: 'Title',
      proposed: 'Paper Lanterns — the clip',
      current: 'Paper Lanterns',
    },
    {
      key: 'description',
      label: 'Description',
      proposed: 'A lantern carried through the town by the river.',
      current: 'Typed by hand.',
    },
  ],
}

let backend: Backend
/** The task keys the backend says are running. */
let running: string[]
/** What waits on the clip's release. */
let waiting: ReleaseProposal[]

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  running = []
  waiting = []
  backend = mockBackend({
    ...answersFor(studio()),
    release_fields: () => FIELDS,
    release_proposals: ({ id }) => (id === IDS.youtube ? waiting : []),
    active_tasks: () => running,
    start_release_task: ({ id, action }) => {
      running = [`${action as string}:release:${id as string}`]
      return {
        chatId: 'chat-meta',
        runId: 'run-meta',
        taskKey: running[0]!,
        title: 'Release meta',
      } satisfies StartedTask
    },
    preview_release_task: () => ({ prompt: 'Write what the clip goes out under.' }),
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

/** The clip's overview, where its one release stands open. */
async function unrolled() {
  const { client } = renderApp(`/works/${IDS.video}`)
  await settled(client)
  const main = await screen.findByRole('main')
  await within(main).findByRole('combobox', { name: en.releases.kind })
  await settled(client)
  return { client, main }
}

describe('the release meta button', () => {
  it('starts the action on this release, says it is writing, and asks again when it ends', async () => {
    const { client, main } = await unrolled()

    await act(async () => {
      fireEvent.click(within(main).getByRole('button', { name: 'Release meta' }))
    })
    await settled(client)

    expect(backend.argsOf('start_release_task')).toEqual([{ id: IDS.youtube, action: ACTION }])
    // The backend's list of running tasks is what the button reads: it is
    // writing, and a second click has nothing to press.
    const writing = within(main).getByRole('button', { name: en.releases.task.writing })
    expect(writing).toBeDisabled()
    expect(within(main).getByText(en.releases.task.writingMeta)).toBeInTheDocument()

    // The answer lands: the fields and what waits beside them are asked again.
    const askedBefore = backend.argsOf('release_proposals').length
    running = []
    waiting = [PROPOSAL]
    await act(async () => {
      await emit('assistant:run', {
        run_id: 'run-meta',
        chat_id: 'chat-meta',
        task: TASK,
        event: { kind: 'finished', body: '' },
      } satisfies RunEmission)
    })
    await settled(client)

    expect(backend.argsOf('release_proposals').length).toBeGreaterThan(askedBefore)
    expect(within(main).getByRole('button', { name: 'Release meta' })).toBeEnabled()
    expect(
      within(main).getByRole('region', { name: en.releases.proposals.label }),
    ).toBeInTheDocument()
    expect(backend.unanswered).toEqual([])
  })

  it('shows what it would send, and starts from there', async () => {
    const { client, main } = await unrolled()

    fireEvent.click(within(main).getByRole('button', { name: `Preview what “Release meta” sends` }))
    const dialog = await screen.findByRole('dialog')
    expect(
      await within(dialog).findByText('Write what the clip goes out under.'),
    ).toBeInTheDocument()
    expect(backend.argsOf('preview_release_task')).toEqual([{ id: IDS.youtube, action: ACTION }])

    await act(async () => {
      fireEvent.click(within(dialog).getByRole('button', { name: en.assistant.previewStart }))
    })
    await settled(client)
    expect(backend.argsOf('start_release_task')).toEqual([{ id: IDS.youtube, action: ACTION }])
  })
})

describe('a proposal waiting on a release', () => {
  beforeEach(() => {
    waiting = [PROPOSAL]
  })

  it('is counted on the head line, and read field by field under the fields', async () => {
    const { main } = await unrolled()
    expect(
      await within(main).findByText(en.releases.proposals.waiting_other.replace('{{count}}', '2')),
    ).toBeInTheDocument()

    const panel = within(main).getByRole('region', { name: en.releases.proposals.label })

    expect(within(panel).getByText(en.releases.proposals.byAssistant)).toBeInTheDocument()
    // What is written now beside what would replace it.
    expect(within(panel).getByText('Typed by hand.')).toBeInTheDocument()
    expect(
      within(panel).getByText('A lantern carried through the town by the river.'),
    ).toBeInTheDocument()
    // Every field taken until one is unchecked.
    expect(within(panel).getAllByRole('checkbox')).toHaveLength(2)
    for (const box of within(panel).getAllByRole('checkbox')) {
      expect(box).toHaveAttribute('aria-checked', 'true')
    }
  })

  it('takes only the fields left checked', async () => {
    backend.answer(
      'apply_proposal',
      ({ messageId }) =>
        ({ message_id: messageId as string, at: NOW, releases: [IDS.youtube] }) as Applied,
    )
    const { client, main } = await unrolled()
    const panel = within(main).getByRole('region', { name: en.releases.proposals.label })

    fireEvent.click(within(panel).getByRole('checkbox', { name: 'Take the proposed Title' }))
    waiting = []
    await act(async () => {
      fireEvent.click(within(panel).getByRole('button', { name: en.releases.proposals.takeChosen }))
    })
    await settled(client)

    expect(backend.argsOf('apply_proposal')).toEqual([
      { messageId: MESSAGE, overrides: { items: ['description'] } },
    ])
    // Taken, it stops waiting.
    expect(within(main).queryByRole('region', { name: en.releases.proposals.label })).toBeNull()
  })

  it('writes nothing when it is turned down', async () => {
    backend.answer('dismiss_proposal', () => {
      waiting = []
    })
    const { client, main } = await unrolled()
    const panel = within(main).getByRole('region', { name: en.releases.proposals.label })

    await act(async () => {
      fireEvent.click(within(panel).getByRole('button', { name: en.releases.proposals.dismiss }))
    })
    await settled(client)

    expect(backend.argsOf('dismiss_proposal')).toEqual([{ messageId: MESSAGE }])
    expect(backend.argsOf('apply_proposal')).toEqual([])
    expect(within(main).queryByRole('region', { name: en.releases.proposals.label })).toBeNull()
  })
})
