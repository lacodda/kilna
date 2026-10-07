import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, within } from '@testing-library/react'
import { mockBackend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * A work's history on one axis (#78): its versions and what each was written
 * from, its scores, what was made from it and when each goes out, latest
 * first and on its own days - with what is booked after today above today.
 */

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  mockBackend(answersFor(studio()))
})

afterEach(() => {
  vi.useRealTimers()
})

async function openHistory() {
  const { client } = renderApp(`/works/${IDS.song}/history`)
  await settled(client)
  const main = await screen.findByRole('main')
  await within(main).findByText(en.timeline.begun)
  return main
}

describe('the History tab', () => {
  it('puts versions, scores, what was made and the day it goes out on one axis', async () => {
    const main = await openHistory()

    const ahead = within(main).getByRole('region', { name: en.timeline.ahead })
    expect(within(ahead).getByText(/Paper Lanterns — audio” goes out on/)).toBeInTheDocument()

    const today = within(main).getByRole('region', { name: en.timeline.today })
    const second = within(today).getByRole('link', { name: /^Lyrics v2 · Second pass · from v1$/ })
    expect(second).toHaveAttribute('href', `/works/${IDS.song}/versions?version=${IDS.lyrics}`)
    expect(within(today).getByRole('link', { name: /^Scored 7.*Lyrics v2$/ })).toBeInTheDocument()

    // The beginning, the first drafts and what was made, a fortnight back.
    expect(within(main).getByRole('link', { name: /^Lyrics v1$/ })).toBeInTheDocument()
    expect(
      within(main).getByRole('link', { name: 'Made “Paper Lanterns (clip)”' }),
    ).toHaveAttribute('href', `/works/${IDS.video}`)
  })

  it('says a score once, from the row, though the journal wrote a line about it too', async () => {
    const main = await openHistory()
    expect(within(main).getAllByText(/^Scored 7/)).toHaveLength(1)
  })

  it('lets a filter take a kind of moment off the axis, but never its beginning', async () => {
    const main = await openHistory()
    const filters = within(main).getByRole('group', { name: en.timeline.filter })
    fireEvent.click(within(filters).getByRole('button', { name: /^Versions/ }))

    expect(within(main).queryByRole('link', { name: /^Lyrics v2/ })).toBeNull()
    expect(within(main).getByText(/^Scored 7/)).toBeInTheDocument()
    expect(within(main).getByText(en.timeline.begun)).toBeInTheDocument()
  })

  it('counts the axis beside the tab, its beginning aside', async () => {
    await openHistory()
    const tabs = screen.getByRole('navigation', { name: en.card.tabs })
    // Three versions, a score, two works made from the song, one release.
    expect(within(tabs).getByRole('link', { name: /History/ })).toHaveTextContent('7')
  })
})
