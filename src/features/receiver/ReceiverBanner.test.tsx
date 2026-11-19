import { describe, expect, it } from 'vitest'
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { emit } from '@tauri-apps/api/event'
import type { ArmedReceiver, ReceiverFound } from '@/lib/api/types'
import { Providers } from '@/app/Providers'
import { createQueryClient } from '@/lib/query/client'
import { mockBackend } from '@/test/backend'
import { ReceiverBanner } from '@/features/receiver/ReceiverBanner'

/*
 * The armed receiver above every screen (v0.156, FR-21 21.2): what waits,
 * put down on a press, and what arrived, moved in on a press.
 */

const ARMED: ArmedReceiver = {
  work_id: 'w1',
  title: 'Dawn',
  role: 'final_audio',
  folder: 'C:/Downloads',
  until: '2030-01-01T10:15:00Z',
}

const FOUND: ReceiverFound = {
  work_id: 'w1',
  title: 'Dawn',
  role: 'final_audio',
  path: 'C:/Downloads/suno-7f3a.mp3',
  name: 'suno-7f3a.mp3',
  taken: null,
}

function mount(armed: ArmedReceiver | null) {
  const backend = mockBackend({
    armed_receiver: () => armed,
    disarm_receiver: () => null,
    take_received: () => ({
      path: 'C:/media/songs/w1/Dawn.mp3',
      relative: 'Dawn.mp3',
      finished: true,
      asset_id: null,
    }),
  })
  render(
    <Providers client={createQueryClient()}>
      <ReceiverBanner />
    </Providers>,
  )
  return backend
}

describe('the receiver bar', () => {
  it('says what waits for which work, and puts it down on a press', async () => {
    const backend = mount(ARMED)

    expect(
      await screen.findByText(/Waiting for the track for “Dawn” in Downloads/),
    ).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }))
    await waitFor(() => expect(backend.argsOf('disarm_receiver')).toHaveLength(1))
  })

  it('says nothing while nothing waits', async () => {
    const backend = mount(null)
    await waitFor(() => expect(backend.argsOf('armed_receiver')).toHaveLength(1))
    expect(screen.queryByRole('status')).toBeNull()
  })

  it('asks before moving in what arrived, and moves it in on a press', async () => {
    const backend = mount(ARMED)
    await screen.findByText(/Waiting for the track/)

    await act(() => emit('receiver-found', FOUND))
    expect(
      await screen.findByText('“suno-7f3a.mp3” arrived: the track for “Dawn”?'),
    ).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Move it in' }))

    await waitFor(() => expect(backend.argsOf('take_received')).toHaveLength(1))
    expect(backend.argsOf('take_received')[0]).toEqual({
      workId: 'w1',
      role: 'final_audio',
      path: 'C:/Downloads/suno-7f3a.mp3',
    })
  })
})
