import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, screen, waitFor, within } from '@testing-library/react'
import type { RepeatMark, Work, WorkPatch } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio, type Studio } from '@/test/workspace'
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
let workspace: Studio

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  workspace = studio()
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
  it('draws the raised marks as their icons, each named by its mark, none of the rest', async () => {
    const header = await openCard(IDS.song)

    // An image named by the mark, not its word on the row (v0.90.1).
    expect(within(header).getByRole('img', { name: 'Working on it' })).toBeInTheDocument()
    expect(within(header).queryByText('Working on it')).toBeNull()
    expect(within(header).queryByRole('img', { name: 'Not sure' })).toBeNull()
    expect(within(header).queryByRole('img', { name: 'The good one' })).toBeNull()
  })

  it('takes a mark off from the menu behind its tile', async () => {
    const header = await openCard(IDS.song)

    fireEvent.click(
      within(header).getByRole('button', {
        name: en.work.marksRaised.replace('{{marks}}', 'Working on it'),
      }),
    )
    const menu = await screen.findByRole('menu')
    const working = within(menu).getByRole('menuitemcheckbox', { name: 'Working on it' })
    expect(working).toHaveAttribute('aria-checked', 'true')
    fireEvent.click(working)

    await waitFor(() => expect(written).toEqual([{ marks: [] }]))
  })

  it('raises a mark from the same menu, keeping the ones already up', async () => {
    const header = await openCard(IDS.song)

    fireEvent.click(
      within(header).getByRole('button', {
        name: en.work.marksRaised.replace('{{marks}}', 'Working on it'),
      }),
    )
    const menu = await screen.findByRole('menu')
    const unsure = within(menu).getByRole('menuitemcheckbox', { name: 'Not sure' })
    expect(unsure).toHaveAttribute('aria-checked', 'false')
    fireEvent.click(unsure)

    await waitFor(() => expect(written).toEqual([{ marks: ['working', 'unsure'] }]))
  })

  it('stands in for the marks with "+ Mark" while none is raised', async () => {
    workspace.works.find((w) => w.id === IDS.song)!.marks = []
    const header = await openCard(IDS.song)

    fireEvent.click(within(header).getByRole('button', { name: en.work.addMark }))
    const menu = await screen.findByRole('menu')
    fireEvent.click(within(menu).getByRole('menuitemcheckbox', { name: 'The good one' }))

    await waitFor(() => expect(written).toEqual([{ marks: ['favourite'] }]))
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

describe('where a work stands, said in its header', () => {
  it('says which publication a song stands on, beside its status', async () => {
    // What the backend would have made of the audio booked for the 22nd.
    workspace.works.find((w) => w.id === IDS.song)!.status = 'scheduled'
    const header = await openCard(IDS.song)

    const chip = await within(header).findByText('Scheduled · as audio Sep 22')
    expect(chip).toHaveAttribute('title', 'Booked as “Paper Lanterns — audio” for Sep 22')
  })

  it('says a status set by hand plainly, whatever went out', async () => {
    workspace.works.find((w) => w.id === IDS.song)!.status = 'shelved'
    const header = await openCard(IDS.song)

    expect(within(header).getByText('Shelved')).toBeInTheDocument()
    expect(within(header).queryByText(/as audio/)).toBeNull()
  })

  it("says an audio's variant by its answer, beside its kind", async () => {
    const header = await openCard(IDS.audio)

    expect(within(header).getByText('Audio')).toBeInTheDocument()
    expect(within(header).getByText('Variant: Original')).toBeInTheDocument()
    // Its status is its own: an audio goes out itself.
    expect(within(header).getByText('Scheduled')).toBeInTheDocument()
  })

  it('names the song a publication goes out for, one click away', async () => {
    const header = await openCard(IDS.audio)

    const back = await within(header).findByRole('link', { name: 'from “Paper Lanterns”' })
    expect(back).toHaveAttribute('href', `/works/${IDS.song}`)
  })

  it('names nothing a song goes out for: it goes out as its publications', async () => {
    const song = await openCard(IDS.song)
    await within(song).findByText('Song')
    expect(within(song).queryByRole('link', { name: /^from / })).toBeNull()
  })

  it("offers a song's publications in its menu, and no second song", async () => {
    const song = await openCard(IDS.song)
    fireEvent.click(within(song).getByRole('button', { name: 'Paper Lanterns' }))
    const menu = await screen.findByRole('menu')
    const offered = within(menu)
      .getAllByRole('menuitem')
      .map((item) => item.textContent)
    expect(offered).toEqual(
      expect.arrayContaining(['Make a video', 'Make an audio', 'Make a short']),
    )
    expect(offered).not.toContain('Make a song')
  })

  it("wears the guard's mark: a song its own, a publication its song's", async () => {
    // The guard of repeats (ADR 0054) answers every work's mark at once; the
    // audio made from the song wears the song's.
    const top = {
      word: 'пульсар',
      level: 'red',
      why: 'rare',
      neighbour_id: 'w-tide',
      neighbour_title: 'Tide',
      day: '2026-09-20',
      booked: false,
      also: 0,
      kept: false,
    } as const
    backend.answer('repeat_marks', () =>
      [IDS.song, IDS.audio].map((workId): RepeatMark => ({
        work_id: workId,
        song_id: IDS.song,
        level: 'red',
        top,
        count: 3,
      })),
    )
    const name = 'Said lately: пульсар — in “Tide”, out Sep 20, and 2 more'

    const song = await openCard(IDS.song)
    const mark = await within(song).findByRole('img', { name })
    expect(mark).toHaveTextContent('3 repeats')
    cleanup()

    const audio = await openCard(IDS.audio)
    expect(await within(audio).findByRole('img', { name })).toBeInTheDocument()
  })
})
