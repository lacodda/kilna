import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, within } from '@testing-library/react'
import type { JournalEntry } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The history's head counts what waits, and its filter shows only that.
 *
 * A warning nobody has marked seen is the one line of history that asks for
 * something: the filter chip carries how many there are, the line carries a
 * dot, and switching to them leaves the ordinary record out.
 */

/** A release that goes out unready, noticed twice and not yet looked at. */
const warning: JournalEntry = {
  id: 'j-unready',
  action: 'release.notReady',
  params: { title: 'Paper Lanterns', date: 'Sep 20' },
  level: 'warn',
  entity: 'work',
  entity_id: IDS.song,
  occurrences: 2,
  created_at: NOW,
  read_at: null,
}

let backend: Backend

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  const workspace = studio()
  workspace.journal = [warning, ...workspace.journal]
  backend = mockBackend({
    ...answersFor(workspace),
    mark_journal_read: () => 1,
  })
})

afterEach(() => {
  vi.useRealTimers()
})

describe('the history', () => {
  it('shows only what needs a look when asked, with its area and its count', async () => {
    const { client } = renderApp('/journal')
    await settled(client)
    const main = await screen.findByRole('main')
    await within(main).findByText('“Harbour Lights” added.')

    const waiting = within(main).getByRole('button', { name: `${en.journal.unreadOnly} · 1` })
    fireEvent.click(waiting)

    const line = (
      await within(main).findByText('“Paper Lanterns” goes out on Sep 20 but is not ready.')
    ).closest('li')!
    expect(within(main).queryByText('“Harbour Lights” added.')).toBeNull()
    expect(within(line).getByText(`${en.journal.kind.release} · 2 times`)).toBeInTheDocument()
    // The dot names itself to a reader who cannot see it.
    expect(within(line).getByText(en.journal.unreadOnly)).toHaveClass('sr-only')
  })

  it('marks everything seen', async () => {
    const { client } = renderApp('/journal')
    await settled(client)
    const main = await screen.findByRole('main')

    fireEvent.click(await within(main).findByRole('button', { name: en.journal.markRead }))
    await settled(client)

    expect(backend.argsOf('mark_journal_read')).toHaveLength(1)
  })
})
