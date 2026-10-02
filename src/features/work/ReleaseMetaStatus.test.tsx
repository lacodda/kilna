import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { MemoryRouter, useLocation } from 'react-router'
import { emit } from '@tauri-apps/api/event'
import type { ReleaseFieldValue, ReleaseProposal, RunEmission, Work } from '@/lib/api/types'
import { Providers } from '@/app/Providers'
import { createQueryClient } from '@/lib/query/client'
import { ProfileContext } from '@/lib/useProfile'
import { mockBackend, type Backend } from '@/test/backend'
import { answersFor, IDS, NOW, studio, type Studio } from '@/test/workspace'
import { ReleaseMetaStatus } from '@/features/work/ReleaseMetaStatus'
import en from '@/i18n/locales/en.json'

/*
 * The bar over a publication's cover and frame (v0.86): while the release
 * meta is being written it says so; when some of it waits for the person it
 * says how much and leads to the release; when it is written it says what;
 * idle, it is not there at all.
 */

const TASK = `release-meta:release:${IDS.audioRelease}`

const WAITING: ReleaseProposal = {
  message_id: 'm-meta',
  chat_id: 'chat-meta',
  client: 'claude-code',
  created_at: NOW,
  fields: [
    { key: 'title', label: 'Title', proposed: 'Paper Lanterns (audio)', current: 'Lanterns' },
    { key: 'tags', label: 'Tags', proposed: 'lanterns, river', current: 'night' },
  ],
}

const FIELDS: ReleaseFieldValue[] = [
  { key: 'title', label: 'Title', type: 'line', value: 'Paper Lanterns', has_template: true },
  { key: 'tags', label: 'Tags', type: 'tags', value: '', has_template: false },
]

let workspace: Studio
let backend: Backend
let running: string[]
let waiting: ReleaseProposal[]

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  workspace = studio()
  running = []
  waiting = []
  backend = mockBackend({
    ...answersFor(workspace),
    active_tasks: () => running,
    release_proposals: ({ id }) => (id === IDS.audioRelease ? waiting : []),
    release_fields: () => FIELDS,
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

/** Where the window is, so a test can see where a button took it. */
function Address() {
  const location = useLocation()
  return <output aria-label="address">{`${location.pathname}${location.search}`}</output>
}

function show(id: string) {
  const work: Work = workspace.works.find((one) => one.id === id)!
  return render(
    <Providers client={createQueryClient()}>
      <MemoryRouter initialEntries={[`/works/${id}/cover`]}>
        <ProfileContext value={workspace.profile}>
          <ReleaseMetaStatus work={work} />
          <Address />
        </ProfileContext>
      </MemoryRouter>
    </Providers>,
  )
}

describe('the release meta bar', () => {
  it('says the meta is being written while it is', async () => {
    running = [TASK]
    show(IDS.audio)
    expect(
      await screen.findByText(en.releases.metaStatus.writing.replace('{{kind}}', 'YouTube')),
    ).toBeInTheDocument()
  })

  it('counts the fields that wait and leads to their release', async () => {
    waiting = [WAITING]
    show(IDS.audio)

    expect(
      await screen.findByText('Release meta for YouTube: 2 fields wait for you'),
    ).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: en.releases.metaStatus.open }))
    expect(screen.getByRole('status', { name: 'address' })).toHaveTextContent(`/works/${IDS.audio}`)
  })

  it('says what was written when the run it watched finishes', async () => {
    running = [TASK]
    show(IDS.audio)
    await screen.findByText(en.releases.metaStatus.writing.replace('{{kind}}', 'YouTube'))

    running = []
    await act(async () => {
      await emit('assistant:run', {
        run_id: 'run-meta',
        chat_id: 'chat-meta',
        task: TASK,
        event: { kind: 'finished', body: '' },
      } satisfies RunEmission)
    })

    // Only the fields that hold something: the tags stayed empty.
    expect(
      await screen.findByText('The release meta for YouTube is ready: Title'),
    ).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: en.releases.metaStatus.close }))
    expect(screen.queryByText(/The release meta for YouTube/)).toBeNull()
  })

  it('draws nothing when nothing is being written or waits', async () => {
    const { container } = show(IDS.audio)
    await waitFor(() => expect(backend.argsOf('release_proposals')).toHaveLength(1))
    expect(container.textContent).toBe(`/works/${IDS.audio}/cover`)
  })

  it('draws nothing on a work that does not go out itself, and asks nothing', async () => {
    const { container } = show(IDS.song)
    await waitFor(() => expect(backend.argsOf('active_tasks')).toHaveLength(1))
    expect(container.textContent).toBe(`/works/${IDS.song}/cover`)
    expect(backend.argsOf('releases_for_work')).toEqual([])
  })
})
