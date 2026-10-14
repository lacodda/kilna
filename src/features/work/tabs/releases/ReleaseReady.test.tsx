import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, screen, within } from '@testing-library/react'
import type { Readiness, ReleaseFieldValue, ScheduledRelease } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * Before and after a release goes out (v0.97): the checklist the place asks
 * for, each gap a way to where it is closed, and the mark that reads where a
 * release went from the link pasted into it.
 */

const FIELDS: ReleaseFieldValue[] = [
  { key: 'title', label: 'Title', type: 'line', value: 'Paper Lanterns', has_template: true },
  { key: 'description', label: 'Description', type: 'text', value: '', has_template: true },
]

/** A clip whose plot and title are there, and whose cover and description
 *  are not. */
const UNREADY: Readiness = {
  checks: [
    { need: { kind: 'role', key: 'plot' }, done: true },
    { need: { kind: 'cover' }, done: false },
    { need: { kind: 'field', key: 'title' }, done: true },
    { need: { kind: 'field', key: 'description' }, done: false },
  ],
  ready: false,
}

let backend: Backend

function workspaceWith(readiness: Readiness) {
  const workspace = studio()
  const answers = answersFor(workspace)
  backend = mockBackend({
    ...answers,
    release_fields: () => FIELDS,
    releases_for_work: (args) =>
      (answers.releases_for_work!(args) as ScheduledRelease[]).map((release) => ({
        ...release,
        readiness,
      })),
  })
  return workspace
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

async function overviewOf(workId: string) {
  const { client } = renderApp(`/works/${workId}`)
  await settled(client)
  const main = await screen.findByRole('main')
  await within(main).findByRole('combobox', { name: en.releases.kind })
  await settled(client)
  return { client, main }
}

describe('the checklist of a release', () => {
  it('lists what the place asks for, a gap a button and what is there a tick', async () => {
    workspaceWith(UNREADY)
    const { main } = await overviewOf(IDS.video)

    const list = within(main).getByRole('list', { name: en.readiness.checklist })
    expect(
      within(list)
        .getAllByRole('listitem')
        .map((item) => item.textContent),
    ).toEqual([
      `Plot${en.readiness.done}`,
      `${en.readiness.cover}${en.readiness.missing}`,
      `Title${en.readiness.done}`,
      `Description${en.readiness.missing}`,
    ])
    // What is there is said, not offered.
    expect(
      within(list)
        .getAllByRole('button')
        .map((button) => button.textContent),
    ).toEqual([
      `${en.readiness.cover}${en.readiness.missing}`,
      `Description${en.readiness.missing}`,
    ])
  })

  it('takes a field to its box, and the cover to the tab it is made on', async () => {
    workspaceWith(UNREADY)
    const { client, main } = await overviewOf(IDS.video)
    const list = within(main).getByRole('list', { name: en.readiness.checklist })

    fireEvent.click(within(list).getByRole('button', { name: /^Description/ }))
    expect(document.activeElement).toBe(within(main).getByRole('textbox', { name: 'Description' }))

    await act(async () => {
      fireEvent.click(
        within(list).getByRole('button', { name: new RegExp(`^${en.readiness.cover}`) }),
      )
    })
    await settled(client)
    expect(await screen.findByText(en.ideas.own)).toBeInTheDocument()
  })

  it('is not drawn once nothing is missing', async () => {
    workspaceWith({ checks: [{ need: { kind: 'role', key: 'plot' }, done: true }], ready: true })
    const { main } = await overviewOf(IDS.video)

    expect(within(main).queryByRole('list', { name: en.readiness.checklist })).toBeNull()
  })
})

describe('marking a release out from its link', () => {
  async function markDialog(workId: string) {
    const { client, main } = await overviewOf(workId)
    fireEvent.click(within(main).getByRole('button', { name: en.releases.actions }))
    fireEvent.click(await screen.findByRole('menuitem', { name: en.calendar.markReleased }))
    const dialog = await screen.findByRole('dialog')
    return { client, dialog }
  }

  it('takes the door the link names, says so, and sends it with the mark', async () => {
    workspaceWith({ checks: [], ready: true })
    const { client, dialog } = await markDialog(IDS.audio)

    expect(
      within(dialog).getByRole('combobox', { name: en.releases.wentThrough }),
    ).toHaveTextContent('YouTube')
    fireEvent.change(within(dialog).getByRole('textbox', { name: en.calendar.urlPrompt }), {
      target: { value: 'https://open.spotify.com/track/1' },
    })
    expect(
      within(dialog).getByRole('combobox', { name: en.releases.wentThrough }),
    ).toHaveTextContent('Streaming')
    expect(within(dialog).getByText(en.releases.recognisedFromLink)).toBeInTheDocument()

    await act(async () => {
      fireEvent.click(within(dialog).getByRole('button', { name: en.calendar.markReleased }))
    })
    await settled(client)
    expect(backend.argsOf('mark_released')).toEqual([
      expect.objectContaining({
        id: IDS.audioRelease,
        url: 'https://open.spotify.com/track/1',
        kind: 'streaming',
      }),
    ])
  })

  it('keeps the planned door for a link that names none, and sends none', async () => {
    workspaceWith({ checks: [], ready: true })
    const { client, dialog } = await markDialog(IDS.audio)

    fireEvent.change(within(dialog).getByRole('textbox', { name: en.calendar.urlPrompt }), {
      target: { value: 'https://example.com/listen' },
    })
    expect(within(dialog).queryByText(en.releases.recognisedFromLink)).toBeNull()

    await act(async () => {
      fireEvent.click(within(dialog).getByRole('button', { name: en.calendar.markReleased }))
    })
    await settled(client)
    expect(backend.argsOf('mark_released')).toEqual([
      expect.objectContaining({ id: IDS.audioRelease, kind: null }),
    ])
  })
})
