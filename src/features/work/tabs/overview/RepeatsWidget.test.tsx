import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, screen, within } from '@testing-library/react'
import type { RepeatFinding, Repeats } from '@/lib/api/types'
import { mockBackend, type Backend, type Handler } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The guard's findings on the overview (ADR 0054): what a song shares with
 * the songs out or booked, loudest first, each with the word, the other song
 * and its day, why it counts - and "I know, keep it", the one thing stored
 * about a finding, said of the song even from a clip made from it.
 */

const finding = (over: Partial<RepeatFinding>): RepeatFinding => ({
  word: 'пульсар',
  level: 'red',
  why: 'rare',
  neighbour_id: IDS.song,
  neighbour_title: 'Paper Lanterns',
  day: '2026-09-22',
  booked: true,
  also: 0,
  kept: false,
  ...over,
})

/** What the guard says about Harbour Lights: an orange spent term, a red
 *  rare word, and a rare word the owner already kept - listed out of order,
 *  as the widget must not rely on being handed them loudest first. */
const harbour: Repeats = {
  work_id: IDS.draft,
  title: 'Harbour Lights',
  level: 'red',
  findings: [
    finding({
      word: 'кофе',
      level: 'orange',
      why: 'register',
      booked: false,
      day: '2026-08-02',
      also: 3,
    }),
    finding({ word: 'пульсар' }),
    finding({ word: 'квазар', kept: true }),
  ],
}

let backend: Backend

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
})

afterEach(() => {
  vi.useRealTimers()
})

async function openOverview(workId: string, repeats: Handler) {
  backend = mockBackend({
    ...answersFor(studio()),
    work_repeats: repeats,
    keep_repeat: () => null,
    unkeep_repeat: () => null,
  })
  const view = renderApp(`/works/${workId}/overview`)
  await settled(view.client)
  const main = await screen.findByRole('main')
  await within(main).findByRole('group', { name: en.card.tab.links })
  await settled(view.client)
  return { ...view, main }
}

describe("the guard's findings on the overview", () => {
  it('lists them loudest first, each with the word, the other song, its day and why', async () => {
    const { main } = await openOverview(IDS.draft, () => harbour)

    const widget = within(main).getByRole('group', { name: en.repeats.caption })
    const rows = within(widget).getAllByRole('listitem')
    expect(rows.map((row) => row.getAttribute('data-level'))).toEqual(['red', 'orange', 'red'])

    const loudest = within(rows[0]!)
    expect(loudest.getByText('пульсар')).toBeInTheDocument()
    expect(loudest.getByText(en.repeats.why.rare)).toBeInTheDocument()
    expect(loudest.getByRole('link', { name: 'Paper Lanterns' })).toHaveAttribute(
      'href',
      `/works/${IDS.song}`,
    )
    expect(loudest.getByText('booked Sep 22')).toBeInTheDocument()

    const spent = within(rows[1]!)
    expect(spent.getByText('кофе')).toBeInTheDocument()
    expect(spent.getByText(en.repeats.why.register)).toBeInTheDocument()
    expect(spent.getByText('out Aug 2')).toBeInTheDocument()
    expect(spent.getByText('+3 songs')).toBeInTheDocument()
    expect(loudest.queryByText(/^\+\d/)).toBeNull()

    // The kept one stays drawn, last, with the way to count it again.
    expect(within(rows[2]!).getByText(en.repeats.kept)).toBeInTheDocument()
    expect(within(rows[2]!).getByRole('button', { name: en.repeats.unkeep })).toBeInTheDocument()
  })

  it('keeps a word of the song, and counts a kept one again', async () => {
    const { client, main } = await openOverview(IDS.draft, () => harbour)
    const widget = within(main).getByRole('group', { name: en.repeats.caption })
    const [loudest, , kept] = within(widget).getAllByRole('listitem')

    await act(async () => {
      fireEvent.click(within(loudest!).getByRole('button', { name: en.repeats.keep }))
    })
    await settled(client)
    expect(backend.argsOf('keep_repeat')).toEqual([{ workId: IDS.draft, word: 'пульсар' }])

    await act(async () => {
      fireEvent.click(within(kept!).getByRole('button', { name: en.repeats.unkeep }))
    })
    await settled(client)
    expect(backend.argsOf('unkeep_repeat')).toEqual([{ workId: IDS.draft, word: 'квазар' }])
  })

  it('keeps a word for the song a clip is made from, and names that song', async () => {
    const song: Repeats = { ...harbour, work_id: IDS.song, title: 'Paper Lanterns' }
    const { client, main } = await openOverview(IDS.video, ({ workId }) =>
      workId === IDS.video ? song : null,
    )

    const widget = within(main).getByRole('group', {
      name: en.repeats.captionOf.replace('{{title}}', 'Paper Lanterns'),
    })
    await act(async () => {
      fireEvent.click(within(widget).getAllByRole('button', { name: en.repeats.keep })[0]!)
    })
    await settled(client)
    expect(backend.argsOf('keep_repeat')).toEqual([{ workId: IDS.song, word: 'пульсар' }])
  })

  it('is not drawn while the song shares nothing', async () => {
    const { main } = await openOverview(IDS.draft, () => null)
    expect(within(main).queryByRole('group', { name: en.repeats.caption })).toBeNull()
  })
})
