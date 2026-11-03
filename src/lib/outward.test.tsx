import { afterEach, describe, expect, it, vi } from 'vitest'
import { onAction, sendNotification, type ActionPerformed } from '@tauri-apps/plugin-notification'
import { emit } from '@tauri-apps/api/event'
import { hearNoticeClicks, inFront, tell, watchPresence } from '@/lib/outward'
import { mockBackend } from '@/test/backend'
import { toastManager } from '@/lib/toast'

vi.mock('@tauri-apps/plugin-notification', () => ({
  sendNotification: vi.fn(),
  onAction: vi.fn(),
}))

afterEach(() => {
  vi.restoreAllMocks()
  vi.mocked(sendNotification).mockReset()
})

function looking(atTheWindow: boolean) {
  vi.spyOn(document, 'hasFocus').mockReturnValue(atTheWindow)
}

describe('saying it to a person who may not be looking', () => {
  it('is a toast with its way to go while the window is in front', () => {
    looking(true)
    const added = vi.spyOn(toastManager, 'add')
    const open = vi.fn()

    tell({ title: 'Finished: Tide', open: { label: 'Open', run: open } })

    expect(sendNotification).not.toHaveBeenCalled()
    expect(added).toHaveBeenCalledWith(
      expect.objectContaining({ title: 'Finished: Tide', actionProps: expect.anything() }),
    )
  })

  it('is the system’s notification while it is not, and a click opens what it was about', async () => {
    looking(false)
    let heard: ((performed: ActionPerformed) => void) | undefined
    const unregister = vi.fn()
    vi.mocked(onAction).mockImplementation(async (handler) => {
      heard = handler
      return { unregister } as never
    })
    const stop = await hearNoticeClicks()
    const added = vi.spyOn(toastManager, 'add')
    const open = vi.fn()

    tell({ title: 'Finished: Tide', body: 'and more', open: { label: 'Open', run: open } })

    expect(added).not.toHaveBeenCalled()
    const [shown] = vi.mocked(sendNotification).mock.calls[0] as [
      { id: number; title: string; body?: string },
    ]
    expect(shown).toMatchObject({ title: 'Finished: Tide', body: 'and more' })

    const tap = (id: number, actionId = 'tap') =>
      heard?.({ actionId, inputValue: null, notification: { id } } as ActionPerformed)
    tap(shown.id + 1000)
    expect(open).not.toHaveBeenCalled()
    tap(shown.id, 'snooze')
    expect(open).not.toHaveBeenCalled()
    tap(shown.id)
    expect(open).toHaveBeenCalledTimes(1)
    // Opened once: the notification is spent.
    tap(shown.id)
    expect(open).toHaveBeenCalledTimes(1)

    stop()
    expect(unregister).toHaveBeenCalled()
  })

  it('believes the system over the page about a window hidden behind the tray', async () => {
    // WebView2 keeps saying "focused" for a hidden window (v0.107 rehearsal).
    looking(true)
    mockBackend({})
    const stop = await watchPresence()
    expect(inFront()).toBe(true)

    await emit('app:background', true)
    expect(inFront()).toBe(false)
    vi.mocked(sendNotification).mockReset()
    tell({ title: 'Finished: Tide' })
    expect(sendNotification).toHaveBeenCalledTimes(1)

    await emit('app:background', false)
    stop()
    expect(inFront()).toBe(true)
  })

  it('a warning in front is a warning toast', () => {
    looking(true)
    const added = vi.spyOn(toastManager, 'add')
    tell({ title: 'Over budget', body: 'this week', warn: true })
    expect(added).toHaveBeenCalledWith(
      expect.objectContaining({ type: 'warning', title: 'Over budget', description: 'this week' }),
    )
  })
})
