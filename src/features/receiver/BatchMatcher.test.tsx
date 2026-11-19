import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { MemoryRouter } from 'react-router'
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import type { MediaMatch } from '@/lib/api/types'
import { Providers } from '@/app/Providers'
import { createQueryClient } from '@/lib/query/client'
import { mockBackend, type Backend } from '@/test/backend'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import { BatchMatcher } from '@/features/receiver/BatchMatcher'

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(() =>
    Promise.resolve([
      'C:/dl/Paper Lanterns.mp3',
      'C:/dl/Paper Lanterns (1).mp3',
      'C:/dl/202604112318.png',
    ]),
  ),
}))

/*
 * A heap of files sorted into works (v0.157, FR-21 21.2 p.2): matched,
 * corrected in a table, and moved only on a press; a duplicate goes to the
 * recycle bin only when the person lets it go.
 */

const MATCHED: MediaMatch[] = [
  {
    path: 'C:/dl/Paper Lanterns.mp3',
    name: 'Paper Lanterns.mp3',
    kind: 'audio',
    work_id: IDS.song,
    work_title: 'Paper Lanterns',
    by: 'title',
    confidence: 0.9,
    ambiguous: [],
    role: 'final_audio',
    duplicate: false,
  },
  {
    path: 'C:/dl/Paper Lanterns (1).mp3',
    name: 'Paper Lanterns (1).mp3',
    kind: 'audio',
    work_id: IDS.song,
    work_title: 'Paper Lanterns',
    by: 'title',
    confidence: 0.8,
    ambiguous: [],
    role: 'candidate_audio',
    duplicate: true,
  },
  {
    path: 'C:/dl/202604112318.png',
    name: '202604112318.png',
    kind: 'image',
    work_id: null,
    work_title: null,
    by: null,
    confidence: null,
    ambiguous: [],
    role: 'final_cover',
    duplicate: false,
  },
]

let backend: Backend

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  backend = mockBackend({
    ...answersFor(studio()),
    match_media: () => MATCHED,
    media_preview: () => [137, 80, 78, 71],
    take_media_batch: () => ({ taken: 1, trashed: 1, failed: [] }),
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

describe('a batch of files sorted into works', () => {
  it('shows what matched and why, and moves only what is placed', async () => {
    render(
      <Providers client={createQueryClient()}>
        <MemoryRouter>
          <BatchMatcher collectionId={null} onClose={() => undefined} />
        </MemoryRouter>
      </Providers>,
    )
    const dialog = await screen.findByRole('dialog', { name: 'Sort files into works' })
    fireEvent.click(within(dialog).getByRole('button', { name: 'Choose files…' }))

    expect(await within(dialog).findByText('Paper Lanterns.mp3')).toBeInTheDocument()
    expect(within(dialog).getByText('by the title · 90%')).toBeInTheDocument()
    expect(within(dialog).getByText('by the title · 80% · a second download')).toBeInTheDocument()
    expect(within(dialog).getByText('Not matched')).toBeInTheDocument()
    // The duplicate waits where it is; the blind picture asks the assistant.
    expect(
      within(dialog).getByRole('button', { name: 'Ask the assistant about 1 picture' }),
    ).toBeInTheDocument()

    fireEvent.click(within(dialog).getByRole('button', { name: 'Sort 1 file into folders' }))
    await waitFor(() => expect(backend.argsOf('take_media_batch')).toHaveLength(1))
    expect(backend.argsOf('take_media_batch')[0]).toEqual({
      picks: [{ path: 'C:/dl/Paper Lanterns.mp3', work_id: IDS.song, role: 'final_audio' }],
      trash: [],
    })
    expect(
      await within(dialog).findByText('Moved in: 1. To the recycle bin: 1.'),
    ).toBeInTheDocument()
  })
})
