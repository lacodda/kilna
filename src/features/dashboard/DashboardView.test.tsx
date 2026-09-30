import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, within } from '@testing-library/react'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The dashboard's buttons go where their names say, and "Heard it" puts away
 * the complaint it stands beside.
 *
 * The studio is changed in two ways: the song's audio release loses its date,
 * so the scored song has nothing booked and becomes a decision to schedule;
 * and the song has not been touched since July, so a stalled-draft complaint
 * stands beside the decisions.
 */

let backend: Backend

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  const workspace = studio()
  workspace.releases = workspace.releases.map((release) =>
    release.id === IDS.audioRelease ? { ...release, scheduled_at: null, status: 'planned' } : release,
  )
  workspace.works = workspace.works.map((work) =>
    work.id === IDS.song ? { ...work, updated_at: '2026-07-01T10:00:00Z' } : work,
  )
  backend = mockBackend({
    ...answersFor(workspace),
    dismiss_finding: ({ key }) => ({ ...(key as object), dismissed_at: NOW }),
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

/** The block of the side column under the caption `title`. */
function widget(title: string): HTMLElement {
  const heading = screen.getByRole('heading', { name: title })
  return heading.closest('header')!.parentElement!
}

describe('the dashboard', () => {
  it('sends a decision to the tab its move is made on', async () => {
    const { client } = renderApp('/dashboard')
    await settled(client)

    fireEvent.click(await screen.findByRole('button', { name: en.dashboard.move.schedule }))
    await settled(client)

    // A song has no Releases tab since v0.86: it is booked through what is
    // made from it, so the move lands on its overview, where they are listed.
    expect(
      await screen.findByRole('link', { name: new RegExp(en.card.tab.overview), current: 'page' }),
    ).toBeTruthy()
  })

  it('puts a complaint away with "Heard it", from the list and from a card', async () => {
    const { client } = renderApp('/dashboard')
    await settled(client)

    const line = (await screen.findByText('“Paper Lanterns” has not moved in a while')).closest(
      'li',
    )!
    fireEvent.click(within(line).getByRole('button', { name: en.findings.dismiss }))
    await settled(client)

    // The card's own is over its cover, named the same way.
    const card = screen.getByRole('heading', { name: 'Paper Lanterns', level: 3 }).parentElement!
      .parentElement!
    fireEvent.click(within(card).getByRole('button', { name: en.findings.dismiss }))
    await settled(client)

    expect(backend.argsOf('dismiss_finding')).toEqual([
      { key: { kind: 'stale-draft', work_id: IDS.song, complaint: 'stale-draft:2' } },
      { key: { kind: 'ready-unscheduled', work_id: IDS.song, complaint: 'ready-unscheduled' } },
    ])
  })

  it('shows what the assistant is running, on which work, and what waits', async () => {
    backend.answer('task_queue', () => ({
      running: [`score:${IDS.draft}`],
      waiting: [`score:${IDS.video}`],
    }))
    const { client } = renderApp('/dashboard')
    await settled(client)

    const running = await vi.waitFor(() => widget(en.dashboard.running))
    expect(within(running).getByText(/Harbour Lights/)).toBeTruthy()
    expect(within(running).getByRole('progressbar')).toBeTruthy()
    expect(
      within(running).getByText(en.assistant.queueWaiting_one.replace('{{count}}', '1')),
    ).toBeTruthy()
  })
})
