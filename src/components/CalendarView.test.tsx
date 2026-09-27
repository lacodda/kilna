import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, screen, within } from '@testing-library/react'
import type { Release } from '@/lib/api'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'

/*
 * Taking a date goes through the contest, never through a plain edit.
 *
 * `update_release` writes a date without asking who holds it, which is right
 * for editing a booking and wrong for claiming one. v0.24 wired the calendar's
 * drag-and-drop to it and shipped a build where dragging one release onto
 * another's day left both there, the rule quietly bypassed. Until v0.77 this
 * was a Rust test reading `CalendarView.tsx` for the name of a mutation; here
 * the release is picked up and carried, and the backend says what it was
 * asked to do.
 */

let backend: Backend

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  const workspace = studio()
  backend = mockBackend({
    ...answersFor(workspace),
    schedule_release: ({ id, slot }) => {
      const release = workspace.releases.find((r) => r.id === id)!
      return { release: { ...release, scheduled_at: slot as string } satisfies Release }
    },
    preview_schedule: () => ({ verdict: 'empty', holder_title: null }),
    update_release: () => {
      throw new Error('a date was written without the contest')
    },
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
  Reflect.deleteProperty(document, 'elementFromPoint')
})

describe('the calendar', () => {
  it('claims the day a release is dropped on through the contest', async () => {
    const { client, container } = renderApp('/calendar')
    await settled(client)
    const main = await screen.findByRole('main')
    const chip = await within(main).findByText('Paper Lanterns', { exact: true })

    // The day the release is let go over: `elementFromPoint` is how the drag
    // finds it, and jsdom lays nothing out, so the answer is given.
    const target = container.querySelector('[data-day="2026-09-24"]')
    expect(target, 'the month shows the 24th').not.toBeNull()
    Object.defineProperty(document, 'elementFromPoint', { value: () => target, configurable: true })

    fireEvent.pointerDown(chip, { button: 0, clientX: 10, clientY: 10 })
    await act(async () => {
      fireEvent.pointerMove(window, { clientX: 60, clientY: 60 })
      fireEvent.pointerUp(window, { clientX: 60, clientY: 60 })
    })
    await settled(client)

    expect(backend.argsOf('schedule_release')).toEqual([{ id: IDS.audio, slot: '2026-09-24' }])
    expect(backend.argsOf('update_release')).toEqual([])
  })
})
