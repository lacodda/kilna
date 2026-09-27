import { afterEach, describe, expect, it, vi } from 'vitest'
import { act, render, waitFor } from '@testing-library/react'
import { emit } from '@tauri-apps/api/event'
import { useQuery } from '@tanstack/react-query'
import type { Run, RunEmission } from '@/lib/api/types'
import { Providers } from '@/app/Providers'
import { createQueryClient } from '@/lib/query/client'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useRunEvent } from '@/lib/runEvents'
import { mockBackend } from '@/test/backend'
import { NOW } from '@/test/workspace'

const RUN: Run = {
  id: 'run-1',
  chat_id: 'chat-1',
  prompt: 'Tighten the chorus',
  state: 'running',
  events: [],
  started_at: NOW,
}

function Probe({ heard }: { heard: (emission: RunEmission) => void }) {
  useQuery({ ...queries.activeTasks(), staleTime: 0 })
  useRunEvent(heard)
  return null
}

afterEach(() => {
  vi.restoreAllMocks()
})

describe("the runs' one ear", () => {
  it('keeps the cache for every screen and tells whoever asked', async () => {
    const backend = mockBackend({ active_tasks: () => [] })
    const client = createQueryClient()
    client.setQueryData<Run[]>(keys.runs('chat-1'), [RUN])
    const heard = vi.fn()
    render(
      <Providers client={client}>
        <Probe heard={heard} />
      </Providers>,
    )
    await waitFor(() => expect(backend.argsOf('active_tasks')).toHaveLength(1))

    await act(async () => {
      await emit('assistant:run', {
        run_id: 'run-1',
        chat_id: 'chat-1',
        event: { kind: 'text', body: 'Cut the second line' },
      } satisfies RunEmission)
    })
    // Every event lands on its run; a block of text moves no list.
    expect(client.getQueryData<Run[]>(keys.runs('chat-1'))![0]!.events).toHaveLength(1)
    expect(heard).toHaveBeenCalledTimes(1)
    expect(backend.argsOf('active_tasks')).toHaveLength(1)

    await act(async () => {
      await emit('assistant:run', {
        run_id: 'run-1',
        chat_id: 'chat-1',
        event: { kind: 'finished', body: 'Cut the second line' },
      } satisfies RunEmission)
    })
    // A run ending moves what is running, for every screen that shows it.
    await waitFor(() => expect(backend.argsOf('active_tasks')).toHaveLength(2))
    expect(client.getQueryData<Run[]>(keys.runs('chat-1'))![0]!.state).toBe('done')
    expect(heard).toHaveBeenCalledTimes(2)
  })
})
