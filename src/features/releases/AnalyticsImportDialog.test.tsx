import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { MemoryRouter } from 'react-router'
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import type { AnalyticsImport } from '@/lib/api/types'
import { Providers } from '@/app/Providers'
import { createQueryClient } from '@/lib/query/client'
import { mockBackend, type Backend } from '@/test/backend'
import { answersFor, NOW, studio } from '@/test/workspace'
import { AnalyticsImportDialog } from '@/features/releases/AnalyticsImportDialog'

/*
 * YouTube Studio's export read before anything is kept (v0.160, FR-22 22.1
 * p.2): what matched which release and how, what matched nothing, one press.
 */

const READ: AnalyticsImport = {
  columns: ['views', 'ctr'],
  rows: [
    {
      line: 3,
      video_id: 'dQw4w9WgXcQ',
      title: 'Lanterns (lyric video)',
      metrics: { views: 1234, ctr: 6.2 },
      release_id: 'r-1',
      work_title: 'Paper Lanterns',
      by: 'id',
    },
    {
      line: 4,
      video_id: 'zzzzzzzzzzz',
      title: 'Someone else’s',
      metrics: { views: 900 },
      release_id: null,
      work_title: null,
      by: null,
    },
  ],
}

let backend: Backend

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  backend = mockBackend({
    ...answersFor(studio()),
    read_analytics: () => READ,
    keep_analytics: () => 1,
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

describe('numbers from YouTube Studio', () => {
  it('shows what matched which release before keeping it on one press', async () => {
    render(
      <Providers client={createQueryClient()}>
        <MemoryRouter>
          <AnalyticsImportDialog path="C:/exports/Table data.csv" onClose={() => undefined} />
        </MemoryRouter>
      </Providers>,
    )
    const dialog = await screen.findByRole('dialog', { name: 'Numbers from YouTube Studio' })
    expect(
      await within(dialog).findByText('1 of 2 videos found among the releases.'),
    ).toBeInTheDocument()
    expect(within(dialog).getByText('→ Paper Lanterns, by its link')).toBeInTheDocument()
    expect(within(dialog).getByText('no release')).toBeInTheDocument()

    fireEvent.click(within(dialog).getByRole('button', { name: 'Keep the numbers of 1 release' }))
    await waitFor(() => expect(backend.argsOf('keep_analytics')).toHaveLength(1))
    expect(backend.argsOf('keep_analytics')[0]).toEqual({ path: 'C:/exports/Table data.csv' })
  })
})
