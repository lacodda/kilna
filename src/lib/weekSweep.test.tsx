import { afterEach, describe, expect, it, vi } from 'vitest'
import { useEffect } from 'react'
import { render, waitFor } from '@testing-library/react'
import { MemoryRouter } from 'react-router'
import type { ScheduledRelease } from '@/lib/api/types'
import { Providers } from '@/app/Providers'
import { createQueryClient } from '@/lib/query/client'
import { toastManager } from '@/lib/toast'
import { ProfileContext } from '@/lib/useProfile'
import { useWeekSweep } from '@/lib/weekSweep'
import { mockBackend } from '@/test/backend'
import { answersFor, studio } from '@/test/workspace'

afterEach(() => {
  vi.restoreAllMocks()
})

function Sweep({ day }: { day: string }) {
  const sweep = useWeekSweep()
  useEffect(() => {
    void sweep(day)
  }, [sweep, day])
  return null
}

describe('the sweep of the week, whoever runs it (v0.107)', () => {
  it('says the release it found first unready for tomorrow - the calendar’s sweep too', async () => {
    vi.spyOn(document, 'hasFocus').mockReturnValue(true)
    const added = vi.spyOn(toastManager, 'add')
    const s = studio()
    const [first] = answersFor(s).calendar!({}) as ScheduledRelease[]
    const release: ScheduledRelease = {
      ...first!,
      scheduled_at: '2026-09-16',
      readiness: { checks: [{ need: { kind: 'cover' }, done: false }], ready: false },
    }
    const backend = mockBackend({
      warn_unready_releases: () => ({ standing: 1, tomorrow: [release] }),
    })

    render(
      <Providers client={createQueryClient()}>
        <MemoryRouter>
          <ProfileContext value={s.profile}>
            <Sweep day="2026-09-15" />
          </ProfileContext>
        </MemoryRouter>
      </Providers>,
    )

    await waitFor(() =>
      expect(added).toHaveBeenCalledWith(
        expect.objectContaining({
          title: `“${release.work_title}” goes out tomorrow, 2026-09-16, and is not ready.`,
        }),
      ),
    )
    expect(backend.argsOf('warn_unready_releases')).toEqual([{ today: '2026-09-15' }])
  })

  it('says nothing when the sweep found nothing new', async () => {
    vi.spyOn(document, 'hasFocus').mockReturnValue(true)
    const added = vi.spyOn(toastManager, 'add')
    const s = studio()
    const backend = mockBackend({
      warn_unready_releases: () => ({ standing: 2, tomorrow: [] }),
    })

    render(
      <Providers client={createQueryClient()}>
        <MemoryRouter>
          <ProfileContext value={s.profile}>
            <Sweep day="2026-09-15" />
          </ProfileContext>
        </MemoryRouter>
      </Providers>,
    )

    await waitFor(() => expect(backend.argsOf('warn_unready_releases')).toHaveLength(1))
    expect(added).not.toHaveBeenCalled()
  })
})
