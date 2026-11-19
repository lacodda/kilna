import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { MemoryRouter } from 'react-router'
import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import type { Applied, MediaProposal } from '@/lib/api/types'
import { Providers } from '@/app/Providers'
import { createQueryClient } from '@/lib/query/client'
import { ProposedMedia } from '@/features/assistant/ProposedMedia'
import { mockBackend, type Backend } from '@/test/backend'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'

/*
 * Files an assistant made beside kilna (v0.156, FR-21 21.1 p.4): read for
 * which work and as what before anything moves, moved in on one press.
 */

const MESSAGE = 'm-media'

let backend: Backend

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  backend = mockBackend({
    ...answersFor(studio()),
    apply_proposal: ({ messageId }) =>
      ({ message_id: messageId as string, at: NOW, versions: [] }) as unknown as Applied,
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

describe('files proposed for the works', () => {
  it('says which file goes to which work as what, and moves them in on a press', async () => {
    const proposal: MediaProposal = {
      kind: 'media',
      items: [
        {
          work_id: IDS.song,
          work_title: 'Paper Lanterns',
          path: 'C:\\Downloads\\suno-7f3a.mp3',
          role: 'final_audio',
          rename: true,
        },
        {
          work_id: IDS.song,
          work_title: 'Paper Lanterns',
          path: 'C:/Downloads/202604112318.png',
          role: 'final_cover',
          rename: true,
        },
      ],
    }
    render(
      <Providers client={createQueryClient()}>
        <MemoryRouter>
          <ProposedMedia messageId={MESSAGE} proposal={proposal} applied={null} dismissed={false} />
        </MemoryRouter>
      </Providers>,
    )

    expect(screen.getByText('2 files for the works')).toBeInTheDocument()
    expect(screen.getByText('suno-7f3a.mp3')).toBeInTheDocument()
    expect(screen.getByText('the cover')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Move 2 files in' }))
    await waitFor(() => expect(backend.argsOf('apply_proposal')).toHaveLength(1))
    expect(backend.argsOf('apply_proposal')[0]).toMatchObject({ messageId: MESSAGE })
  })
})
