import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, screen, within } from '@testing-library/react'
import type { RepeatMark } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The catalogue as the mockup lays it out (v0.79): a toolbar of two rows with
 * no dropdown for the status or the tier, the table in a panel with the
 * columns a release is read by, and the bulk bar at the foot with its acts
 * named on buttons.
 *
 * The bar is the part worth walking. It used to be inserted above the table,
 * so ticking the first box moved every row down by the bar's height and the
 * next click landed a row lower than it was aimed; and moving a batch to
 * another status sat behind a menu called "Actions". Here a box is ticked,
 * the bar is found after the table rather than before it, and its buttons
 * reach the backend with exactly what was ticked.
 */

let backend: Backend

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  backend = mockBackend({
    ...answersFor(studio()),
    unschedule_works: ({ workIds }) => ({ changed: (workIds as string[]).length, skipped: 0 }),
    set_works_status: ({ workIds }) => ({ changed: (workIds as string[]).length, skipped: 0 }),
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

/** The catalogue, loaded, and the row a title stands on. */
async function open() {
  const { client } = renderApp('/catalogue')
  await settled(client)
  const main = await screen.findByRole('main')
  await within(main).findByText('Harbour Lights', { exact: true })
  await settled(client)
  const rowOf = (title: string) => within(main).getByText(title, { exact: true }).closest('tr')!
  return { client, main, rowOf }
}

describe('the catalogue', () => {
  it('narrows by status and tier without a dropdown for either', async () => {
    const { main } = await open()

    // The query box says `status:` and `tier:`, and the columns' funnels tick
    // them: a dropdown for each beside those was a third way to say one thing.
    expect(within(main).queryAllByRole('combobox')).toEqual([])
    expect(within(main).getByRole('textbox', { name: en.works.search })).toBeInTheDocument()

    const headings = within(main)
      .getAllByRole('columnheader')
      .map((cell) => cell.textContent)
    expect(headings).toEqual(
      expect.arrayContaining([
        en.catalogue.column.status,
        en.catalogue.column.release,
        en.catalogue.column.ready,
      ]),
    )
  })

  it('reads each work down to its next release and whether it can go out', async () => {
    const { rowOf } = await open()

    // The song is booked for the 22nd, and the studio's releases lack nothing.
    const song = within(rowOf('Paper Lanterns'))
    expect(song.getByText('Sep 22')).toBeInTheDocument()
    expect(song.getByText(en.calendar.ready)).toBeInTheDocument()
    expect(song.getByText('Scored')).toBeInTheDocument()

    // Nothing booked: no date, and nothing to be ready.
    const draft = within(rowOf('Harbour Lights'))
    expect(draft.queryByText(en.calendar.ready)).toBeNull()
    expect(draft.getByText('Draft')).toBeInTheDocument()
  })

  it('says which value the profile lacks, in place of the count', async () => {
    const { main } = await open()
    const box = within(main).getByRole('textbox', { name: en.works.search })

    await act(async () => {
      fireEvent.change(box, { target: { value: 'tier:nonesuch' } })
    })

    expect(box).toHaveAttribute('aria-invalid', 'true')
    expect(within(main).getByRole('status')).toHaveTextContent(
      'This profile has no tier called "nonesuch".',
    )
  })

  it('brings the bulk bar up under the table and acts on what was ticked', async () => {
    const { client, main, rowOf } = await open()
    const bar = () => screen.queryByRole('toolbar', { name: en.catalogue.bulk.label })
    expect(bar(), 'no bar while nothing is ticked').toBeNull()

    await act(async () => {
      fireEvent.click(
        within(rowOf('Harbour Lights')).getByRole('checkbox', { name: 'Select Harbour Lights' }),
      )
    })

    const shown = bar()
    expect(shown).not.toBeNull()
    // After the table in the document, not before it: the rows keep their
    // place when the first box is ticked.
    const table = within(main).getByRole('table')
    expect(table.compareDocumentPosition(shown!) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    expect(table.contains(shown!)).toBe(false)
    expect(within(main).getByText('Harbour Lights', { exact: true }).closest('tr')).toHaveAttribute(
      'aria-selected',
      'true',
    )

    // Every act has its own button, the risky one included.
    const inBar = within(shown!)
    expect(inBar.getByRole('button', { name: en.catalogue.action.delete })).toBeInTheDocument()
    expect(inBar.getByRole('button', { name: en.catalogue.clearSelection })).toBeInTheDocument()

    await act(async () => {
      fireEvent.click(inBar.getByRole('button', { name: en.catalogue.bulk.unschedule }))
    })
    await settled(client)

    expect(backend.argsOf('unschedule_works')).toEqual([{ workIds: [IDS.draft] }])
    // Done with: the ticks go, and the bar with them.
    expect(bar()).toBeNull()
  })

  it("marks a song the guard holds against one booked, and names why in the mark's words", async () => {
    // The guard of repeats (ADR 0054): Harbour Lights has not gone out, and
    // says a rare word Paper Lanterns says on the 22nd - and one more.
    backend.answer('repeat_marks', () => [
      {
        work_id: IDS.draft,
        song_id: IDS.draft,
        level: 'red',
        top: {
          word: 'пульсар',
          level: 'red',
          why: 'rare',
          neighbour_id: IDS.song,
          neighbour_title: 'Paper Lanterns',
          day: '2026-09-22',
          booked: true,
          also: 0,
          kept: false,
        },
        count: 2,
      } satisfies RepeatMark,
    ])
    const { rowOf } = await open()

    // The hint is the mark's name: the level, the word, the other song and
    // its day, and how many more - what a reader who cannot see red hears.
    const mark = await within(rowOf('Harbour Lights')).findByRole('img', {
      name: 'Said lately: пульсар — in “Paper Lanterns”, booked Sep 22, and 1 more',
    })
    expect(mark).toHaveAttribute('data-repeat', 'red')
    expect(mark).toHaveTextContent('2')
    // A song nothing is held against wears no mark.
    expect(within(rowOf('Paper Lanterns')).queryByRole('img', { name: /Said/ })).toBeNull()
  })
})
