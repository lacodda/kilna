import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { screen, within } from '@testing-library/react'
import type { Derived, Links } from '@/lib/api/types'
import { mockBackend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'

/*
 * What was made from a work says where its releases are.
 *
 * "When did the clip come out" is asked on the song's card; until v0.80 the
 * answer was only on the clip's own card or in the calendar. The fake studio
 * is on 15 September, so a day before it that nothing went out on is late.
 */

/** A work made from the song, with its releases standing as `releases` says. */
function made(
  id: string,
  title: string,
  releases: Pick<Derived, 'released' | 'last_released_at' | 'next_scheduled_at'>,
): Derived {
  return {
    link_id: `l-${id}`,
    work_id: id,
    title,
    kind: 'video',
    status: 'draft',
    role: 'donor',
    created_at: NOW,
    ...releases,
  }
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  const links: Links = {
    sources: [],
    derived: [
      made('w-twice', 'Out twice', {
        released: 2,
        last_released_at: '2026-09-12T09:00:00Z',
        next_scheduled_at: '2026-10-03',
      }),
      made('w-once', 'Out once', {
        released: 1,
        last_released_at: '2026-09-02T09:00:00Z',
        next_scheduled_at: null,
      }),
      made('w-late', 'Running late', {
        released: 0,
        last_released_at: null,
        next_scheduled_at: '2026-09-10',
      }),
      made('w-quiet', 'Nothing planned', {
        released: 0,
        last_released_at: null,
        next_scheduled_at: null,
      }),
    ],
  }
  mockBackend({ ...answersFor(studio()), list_links: () => links })
})

afterEach(() => {
  vi.useRealTimers()
})

/** The row of the work called `title`. */
async function rowOf(title: string): Promise<HTMLElement> {
  const main = await screen.findByRole('main')
  const words = await within(main).findByText(title, { exact: true })
  return words.closest('li')!
}

describe('what was made from this', () => {
  it('says when its releases went out, and when the next one goes', async () => {
    const { client } = renderApp(`/works/${IDS.song}/links`)
    await settled(client)

    const twice = await rowOf('Out twice')
    expect(within(twice).getByText('2 out · last Sep 12')).toBeInTheDocument()
    expect(within(twice).getByText('Planned for Oct 3')).toBeInTheDocument()

    expect(within(await rowOf('Out once')).getByText('Out Sep 2')).toBeInTheDocument()
    expect(within(await rowOf('Running late')).getByText('Late: due Sep 10')).toBeInTheDocument()

    // Nothing out and nothing planned: no chip saying so.
    const quiet = await rowOf('Nothing planned')
    expect(within(quiet).queryByText(/out|Planned|Late/)).toBeNull()
  })
})
