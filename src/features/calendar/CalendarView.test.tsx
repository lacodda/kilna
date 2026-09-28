import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, screen, within } from '@testing-library/react'
import type { Release } from '@/lib/api/types'
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
    unschedule_release: ({ id }) => {
      const release = workspace.releases.find((r) => r.id === id)!
      return { ...release, scheduled_at: null, status: 'planned' } satisfies Release
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

/**
 * Pick up the booked release and let go of it over whatever `selector` finds.
 *
 * `elementFromPoint` is how the drag finds where it landed, and jsdom lays
 * nothing out, so the answer is given.
 */
async function carryTo(selector: string) {
  const { client, container } = renderApp('/calendar')
  await settled(client)
  const main = await screen.findByRole('main')
  const chip = await within(main).findByText('Paper Lanterns', { exact: true })

  const target = container.querySelector(selector)
  expect(target, `the screen shows ${selector}`).not.toBeNull()
  Object.defineProperty(document, 'elementFromPoint', { value: () => target, configurable: true })

  fireEvent.pointerDown(chip, { button: 0, clientX: 10, clientY: 10 })
  await act(async () => {
    fireEvent.pointerMove(window, { clientX: 60, clientY: 60 })
    fireEvent.pointerUp(window, { clientX: 60, clientY: 60 })
  })
  await settled(client)
}

describe('the calendar', () => {
  it('claims the day a release is dropped on through the contest', async () => {
    await carryTo('[data-day="2026-09-24"]')

    expect(backend.argsOf('schedule_release')).toEqual([{ id: IDS.audio, slot: '2026-09-24' }])
    expect(backend.argsOf('update_release')).toEqual([])
  })

  it('writes nothing when a release is let go over the day it holds', async () => {
    // Picked up and put back: no date written, no "moved" said.
    await carryTo('[data-day="2026-09-22"]')

    expect(backend.argsOf('schedule_release')).toEqual([])
  })

  it('holds the month still while a release is held over the queue', async () => {
    // The month turns while a chip rests past its edge, and the queue stands
    // past its right edge. jsdom lays nothing out, so every pointer is past
    // it here - as the queue is in the window.
    const { client, container } = renderApp('/calendar')
    await settled(client)
    const main = await screen.findByRole('main')
    const chip = await within(main).findByText('Paper Lanterns', { exact: true })
    const queue = container.querySelector('[data-queue-drop]')!
    Object.defineProperty(document, 'elementFromPoint', { value: () => queue, configurable: true })

    fireEvent.pointerDown(chip, { button: 0, clientX: 10, clientY: 10 })
    await act(async () => {
      fireEvent.pointerMove(window, { clientX: 60, clientY: 60 })
    })
    fireEvent.pointerEnter(queue)
    // Two turns' worth of resting: without the hold, November.
    await act(() => new Promise((resolve) => setTimeout(resolve, 1_200)))
    expect(within(main).getByRole('heading', { level: 1 })).toHaveTextContent('September 2026')

    await act(async () => {
      fireEvent.pointerUp(window, { clientX: 60, clientY: 60 })
    })
    await settled(client)
    expect(backend.argsOf('unschedule_release')).toEqual([{ id: IDS.audio }])
  })

  it('returns a release to the queue when it is let go over the queue', async () => {
    // Anywhere on the queue, not only its foot: a row of the list is where
    // a carried chip is most likely to be let go.
    await carryTo('[data-queue-drop] li')

    expect(backend.argsOf('unschedule_release')).toEqual([{ id: IDS.audio }])
    expect(backend.argsOf('schedule_release')).toEqual([])
  })
})
