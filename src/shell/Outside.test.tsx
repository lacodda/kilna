import { afterEach, describe, expect, it, vi } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import i18n from '@/i18n'
import type { ScheduledRelease } from '@/lib/api/types'
import { tomorrowNotice } from '@/shell/Outside'
import { mockBackend } from '@/test/backend'
import { renderApp } from '@/test/render'
import { answersFor, studio } from '@/test/workspace'

const t = i18n.t.bind(i18n)

afterEach(() => {
  vi.restoreAllMocks()
})

/** A release of the studio, due on `date` and lacking its cover. */
function unready(date: string): ScheduledRelease {
  const s = studio()
  const [first] = answersFor(s).calendar!({}) as ScheduledRelease[]
  return {
    ...first!,
    scheduled_at: date,
    readiness: { checks: [{ need: { kind: 'cover' }, done: false }], ready: false },
  }
}

describe('the day before a slot (v0.107)', () => {
  it('names the release, its date and what it lacks', () => {
    const release = unready('2026-09-16')
    const go = vi.fn()
    const notice = tomorrowNotice([release], studio().profile.config, t, go)

    expect(notice.title).toBe(
      `“${release.work_title}” goes out tomorrow, 2026-09-16, and is not ready.`,
    )
    expect(notice.body).toMatch(/^Missing: /)
    expect(notice.warn).toBe(true)
    notice.open?.run()
    expect(go).toHaveBeenCalledWith(`/works/${release.work_id}`)
  })

  it('counts several and opens the calendar', () => {
    const go = vi.fn()
    const notice = tomorrowNotice(
      [unready('2026-09-16'), unready('2026-09-16')],
      studio().profile.config,
      t,
      go,
    )
    expect(notice.title).toBe('2 releases go out tomorrow and are not ready')
    notice.open?.run()
    expect(go).toHaveBeenCalledWith('/calendar')
  })

  it('is said when the sweep at start finds it first, and the tray learns its words', async () => {
    vi.spyOn(document, 'hasFocus').mockReturnValue(true)
    const s = studio()
    const release = unready('2026-09-16')
    const backend = mockBackend({
      ...answersFor(s),
      warn_unready_releases: () => ({ standing: 1, tomorrow: [release] }),
    })
    renderApp('/dashboard')

    expect(
      await screen.findByText(
        `“${release.work_title}” goes out tomorrow, 2026-09-16, and is not ready.`,
      ),
    ).toBeInTheDocument()
    await waitFor(() =>
      expect(backend.argsOf('set_tray_words')).toContainEqual({
        said: { open: 'Open kilna', quit: 'Quit kilna' },
      }),
    )
  })
})
