import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, waitFor, within } from '@testing-library/react'
import type { Work, WorkPatch } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The card's header: the marks raised on a work, the way to raise another,
 * and the numbers beside the tabs.
 *
 * Until v0.80 every mark the profile knows stood in the header as a switch,
 * raised or not, and four of the tabs' numbers came from four whole lists.
 * These open a card against the fake studio, where "Paper Lanterns" carries
 * one mark ("Working on it") of the three the Studio profile defines.
 */

let backend: Backend
let written: WorkPatch[]

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  const workspace = studio()
  written = []
  backend = mockBackend({
    ...answersFor(workspace),
    update_work: ({ id, patch }) => {
      written.push(patch as WorkPatch)
      const work = workspace.works.find((w) => w.id === id)!
      return { ...work, ...(patch as Partial<Work>) }
    },
  })
})

afterEach(() => {
  vi.useRealTimers()
})

/** The header of the card for `workId`, once everything it asks has answered. */
async function openCard(workId: string): Promise<HTMLElement> {
  const { client } = renderApp(`/works/${workId}/overview`)
  await settled(client)
  const title = await screen.findByRole('heading', { level: 1, name: /^Paper Lanterns/ })
  await settled(client)
  return title.closest('header')!
}

describe("the card's header", () => {
  it('draws only the marks that are raised, each with a way to take it off', async () => {
    const header = await openCard(IDS.song)

    expect(within(header).getByText('Working on it')).toBeInTheDocument()
    expect(within(header).queryByText('Not sure')).toBeNull()
    expect(within(header).queryByText('The good one')).toBeNull()

    fireEvent.click(within(header).getByRole('button', { name: 'Take off "Working on it"' }))
    await waitFor(() => expect(written).toEqual([{ marks: [] }]))
  })

  it('raises a mark from the menu beside the tags, keeping the ones already up', async () => {
    const header = await openCard(IDS.song)

    fireEvent.click(within(header).getByRole('button', { name: en.work.addMark }))
    const menu = await screen.findByRole('menu')
    // Only what is not raised yet is offered.
    expect(within(menu).queryByText('Working on it')).toBeNull()
    fireEvent.click(within(menu).getByRole('menuitem', { name: 'Not sure' }))

    await waitFor(() => expect(written).toEqual([{ marks: ['working', 'unsure'] }]))
  })

  it('numbers every tab from one answer, and marks the comments that wait', async () => {
    const header = await openCard(IDS.video)
    const tabs = within(header).getByRole('navigation', { name: en.card.tabs })

    expect(backend.argsOf('card_counts')).toContainEqual({ workId: IDS.video })
    // The video was made from the song, and its one comment waits.
    expect(within(tabs).getByRole('link', { name: /^Links\s*1$/ })).toBeInTheDocument()
    const comments = within(tabs).getByRole('link', { name: /^Comments\s*1$/ })
    expect(within(comments).getByText('1')).toHaveAttribute('title')
    // Nothing to count, nothing beside it.
    expect(within(tabs).getByRole('link', { name: /^Overview$/ })).toBeInTheDocument()
  })
})
