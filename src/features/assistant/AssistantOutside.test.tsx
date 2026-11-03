import { afterEach, describe, expect, it, vi } from 'vitest'
import { act, render, waitFor } from '@testing-library/react'
import { MemoryRouter } from 'react-router'
import { emit } from '@tauri-apps/api/event'
import { sendNotification } from '@tauri-apps/plugin-notification'
import type { Activity, RunEmission, SpendCheck } from '@/lib/api/types'
import { Providers } from '@/app/Providers'
import { AssistantLauncher } from '@/features/assistant/AssistantDrawer'
import { createQueryClient } from '@/lib/query/client'
import { toastManager } from '@/lib/toast'
import { mockBackend, type Handler } from '@/test/backend'

vi.mock('@tauri-apps/plugin-notification', () => ({
  sendNotification: vi.fn(),
  onAction: vi.fn(),
}))

const UNDER: SpendCheck = { spend: { today: 0.2, week: 1, budget: 10 }, crossed: false }

function mount(busy: Activity, handlers: Record<string, Handler> = {}) {
  const backend = mockBackend({
    active_runs: () => [],
    activity: () => busy,
    check_spend: () => UNDER,
    ...handlers,
  })
  render(
    <Providers client={createQueryClient()}>
      <MemoryRouter>
        <AssistantLauncher />
      </MemoryRouter>
    </Providers>,
  )
  return backend
}

const ended = (kind: 'finished' | 'failed', task?: string): RunEmission => ({
  run_id: `run-${kind}`,
  chat_id: 'chat-1',
  task,
  event: kind === 'finished' ? { kind, body: 'A shorter chorus.' } : { kind, message: 'no' },
})

const shownTitles = () =>
  vi.mocked(sendNotification).mock.calls.map(([options]) => (options as { title: string }).title)

afterEach(() => {
  vi.restoreAllMocks()
  vi.mocked(sendNotification).mockReset()
})

describe('a run ending while nobody is looking (v0.107)', () => {
  it('is said by the system at once when nothing else is going', async () => {
    vi.spyOn(document, 'hasFocus').mockReturnValue(false)
    const backend = mount({ running: 0, queued: 0 })
    await waitFor(() => expect(backend.argsOf('activity')).toHaveLength(1))

    await act(() => emit('assistant:run', ended('finished', 'critique:w1')))

    expect(shownTitles()).toEqual(['The assistant finished.'])
    // And the week's spend is looked at, since the run was priced.
    await waitFor(() => expect(backend.argsOf('check_spend')).toHaveLength(1))
  })

  it('is held while a batch goes on, and said together when the last ends', async () => {
    vi.spyOn(document, 'hasFocus').mockReturnValue(false)
    const backend = mount({ running: 2, queued: 3 })
    await waitFor(() => expect(backend.argsOf('activity')).toHaveLength(1))

    await act(() => emit('assistant:run', ended('finished', 'critique:w1')))
    await act(() => emit('assistant:run', ended('failed', 'critique:w2')))
    expect(sendNotification).not.toHaveBeenCalled()

    await act(() => emit('app:activity', { running: 0, queued: 0 } satisfies Activity))

    await waitFor(() => expect(shownTitles()).toEqual(['1 answer is ready']))
  })

  it('counts a question typed before the window was closed', async () => {
    vi.spyOn(document, 'hasFocus').mockReturnValue(false)
    mount({ running: 0, queued: 0 })

    await act(() => emit('assistant:run', ended('finished')))

    await waitFor(() => expect(shownTitles()).toEqual(['The assistant answered.']))
  })

  it('is a toast, as before, while the window is in front - and nothing outside', async () => {
    vi.spyOn(document, 'hasFocus').mockReturnValue(true)
    const added = vi.spyOn(toastManager, 'add')
    mount({ running: 0, queued: 0 })

    await act(() => emit('assistant:run', ended('finished', 'critique:w1')))
    // A typed question in front of the window is answered on screen.
    await act(() => emit('assistant:run', ended('finished')))

    expect(added).toHaveBeenCalledTimes(1)
    expect(added).toHaveBeenCalledWith(
      expect.objectContaining({ title: 'The assistant finished.' }),
    )
    expect(sendNotification).not.toHaveBeenCalled()
  })

  it('says a week gone over its budget, once it is seen', async () => {
    vi.spyOn(document, 'hasFocus').mockReturnValue(false)
    mount(
      { running: 0, queued: 0 },
      {
        check_spend: () => ({ spend: { today: 2, week: 12.4, budget: 10 }, crossed: true }),
      },
    )

    await act(() => emit('assistant:run', ended('finished', 'critique:w1')))

    await waitFor(() =>
      expect(shownTitles()).toContain('This week’s runs cost $12.40, over the $10.00 budget.'),
    )
  })
})
