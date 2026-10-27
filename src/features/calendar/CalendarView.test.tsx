import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, screen, within } from '@testing-library/react'
import type { Release, RepeatFinding, RepeatMark, ShiftPlan, TimeOff } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { noScrollerInAnother } from '@/test/regions'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

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
    preview_schedule: () => ({ verdict: 'empty', holder_title: null, repeats: [], closed: null }),
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
  const chip = await within(main).findByText('Paper Lanterns — audio', { exact: true })

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
  it('asks which work goes out on a day in a list the wheel reaches', async () => {
    // The day's plus opens the choice of a work - the dialog every other
    // "which work" question shares. Its list stood in a scrolling column of
    // its own until v0.90.1, too short to scroll inside it, and the wheel
    // stopped on it.
    const { client } = renderApp('/calendar')
    await settled(client)
    const main = await screen.findByRole('main')
    const [plus] = await within(main).findAllByRole('button', { name: en.calendar.addOnDay })
    fireEvent.click(plus!)

    const dialog = await screen.findByRole('dialog', {
      name: (name) => name.startsWith(en.calendar.fillDay.split('{{')[0]!),
    })
    await within(dialog).findByText('Harbour Lights')
    noScrollerInAnother(dialog, 'the choice of a work')
  })

  it('claims the day a release is dropped on through the contest', async () => {
    await carryTo('[data-day="2026-09-24"]')

    expect(backend.argsOf('schedule_release')).toEqual([
      { id: IDS.audioRelease, slot: '2026-09-24' },
    ])
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
    const chip = await within(main).findByText('Paper Lanterns — audio', { exact: true })
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
    expect(backend.argsOf('unschedule_release')).toEqual([{ id: IDS.audioRelease }])
  })

  it('warns before and as a release lands on a day it repeats a song on, and lands it', async () => {
    // The guard of repeats (ADR 0054): on the 24th the audio release's song
    // would say a rare word "Tide" said on the 20th. A warning, not a refusal.
    const repeat = {
      word: 'пульсар',
      level: 'red',
      why: 'rare',
      neighbour_id: 'w-tide',
      neighbour_title: 'Tide',
      day: '2026-09-20',
      booked: false,
      also: 0,
      kept: false,
    } satisfies RepeatFinding
    backend.answer('preview_schedule', ({ slot }) => ({
      verdict: 'empty',
      holder_title: null,
      repeats: slot === '2026-09-24' ? [repeat] : [],
      closed: null,
    }))
    const warning = 'Too close: пульсар — in “Tide”, out Sep 20'

    const { client, container } = renderApp('/calendar')
    await settled(client)
    const main = await screen.findByRole('main')
    const chip = await within(main).findByText('Paper Lanterns — audio', { exact: true })
    const day = container.querySelector<HTMLElement>('[data-day="2026-09-24"]')!
    Object.defineProperty(document, 'elementFromPoint', { value: () => day, configurable: true })
    // The chip rests over the day while the dry run answers. jsdom lays
    // nothing out, so every pointer is past the month's edge and the month
    // would turn under a chip held long enough; a month laid out wide keeps
    // the pointer in its middle, as it is in the window.
    const wide = { left: 0, top: 0, right: 1_000, bottom: 800, width: 1_000, height: 800 }
    vi.spyOn(Element.prototype, 'getBoundingClientRect').mockReturnValue({
      ...wide,
      x: 0,
      y: 0,
      toJSON: () => wide,
    })

    // Held over the day: the day says it, before anything is written.
    fireEvent.pointerDown(chip, { button: 0, clientX: 300, clientY: 300 })
    await act(async () => {
      fireEvent.pointerMove(window, { clientX: 400, clientY: 400 })
    })
    fireEvent.pointerEnter(day)
    expect(await within(day).findByText(warning)).toBeInTheDocument()
    expect(backend.argsOf('schedule_release')).toEqual([])

    // Let go: it lands all the same, and the toast says what it repeats.
    await act(async () => {
      fireEvent.pointerUp(window, { clientX: 400, clientY: 400 })
    })
    await settled(client)
    expect(backend.argsOf('schedule_release')).toEqual([
      { id: IDS.audioRelease, slot: '2026-09-24' },
    ])
    // Said by the toast now - the day's line went with the chip in the air.
    expect(within(day).queryByText(warning)).toBeNull()
    expect(await screen.findByText(warning)).toBeInTheDocument()
    expect(screen.getByText(en.toast.releaseMoved)).toBeInTheDocument()
  })

  it("wears its song's mark on the chip and says it in the chip's card", async () => {
    backend.answer('repeat_marks', () => [
      {
        work_id: IDS.audio,
        song_id: IDS.song,
        level: 'orange',
        top: {
          word: 'кофе',
          level: 'orange',
          why: 'register',
          neighbour_id: 'w-tide',
          neighbour_title: 'Tide',
          day: '2026-08-02',
          booked: false,
          also: 0,
          kept: false,
        },
        count: 1,
      } satisfies RepeatMark,
    ])
    const { client } = renderApp('/calendar')
    await settled(client)
    const main = await screen.findByRole('main')
    const chip = (await within(main).findByText('Paper Lanterns — audio', { exact: true })).closest(
      'button',
    )!

    expect(
      await within(chip).findByRole('img', {
        name: 'Said before: spent: кофе — in “Tide”, out Aug 2',
      }),
    ).toHaveAttribute('data-repeat', 'orange')
  })

  it('returns a release to the queue when it is let go over the queue', async () => {
    // Anywhere on the queue, not only its foot: a row of the list is where
    // a carried chip is most likely to be let go.
    await carryTo('[data-queue-drop] li')

    expect(backend.argsOf('unschedule_release')).toEqual([{ id: IDS.audioRelease }])
    expect(backend.argsOf('schedule_release')).toEqual([])
  })
})

/** A span of closed days, as the backend lists it. */
function span(id: string, from: string, to: string, label: string): TimeOff {
  return {
    id,
    profile_id: 'p',
    starts_on: from,
    ends_on: to,
    label,
    created_at: NOW,
    updated_at: NOW,
  }
}

describe('days and shifts (v0.96)', () => {
  it('shows a closed day by its name, and lands a release there with the closure said', async () => {
    backend.answer('list_time_off', () => [span('t1', '2026-09-24', '2026-09-25', 'Away')])
    backend.answer('preview_schedule', ({ slot }) => ({
      verdict: 'empty',
      holder_title: null,
      repeats: [],
      closed: slot === '2026-09-24' ? 'Away' : null,
    }))
    const { client, container } = renderApp('/calendar')
    await settled(client)
    const day = container.querySelector<HTMLElement>('[data-day="2026-09-24"]')!
    expect(within(day).getByText('Away')).toBeInTheDocument()
    expect(container.querySelector('[data-day="2026-09-23"]')).not.toHaveTextContent('Away')

    const main = await screen.findByRole('main')
    const chip = await within(main).findByText('Paper Lanterns — audio', { exact: true })
    Object.defineProperty(document, 'elementFromPoint', { value: () => day, configurable: true })
    const wide = { left: 0, top: 0, right: 1_000, bottom: 800, width: 1_000, height: 800 }
    vi.spyOn(Element.prototype, 'getBoundingClientRect').mockReturnValue({
      ...wide,
      x: 0,
      y: 0,
      toJSON: () => wide,
    })
    fireEvent.pointerDown(chip, { button: 0, clientX: 300, clientY: 300 })
    await act(async () => {
      fireEvent.pointerMove(window, { clientX: 400, clientY: 400 })
    })
    fireEvent.pointerEnter(day)
    expect(
      await within(day).findByText(en.timeOff.landsOnClosed.replace('{{title}}', 'Away')),
    ).toBeInTheDocument()

    await act(async () => {
      fireEvent.pointerUp(window, { clientX: 400, clientY: 400 })
    })
    await settled(client)
    // Closed to the layout, not to a person.
    expect(backend.argsOf('schedule_release')).toEqual([
      { id: IDS.audioRelease, slot: '2026-09-24' },
    ])
  })

  it('draws one week of seven days, and the agenda folds the empty days', async () => {
    const { client, container } = renderApp('/calendar')
    await settled(client)
    const main = await screen.findByRole('main')

    fireEvent.click(within(main).getByRole('radio', { name: en.calendar.view.week }))
    await settled(client)
    const week = [...container.querySelectorAll<HTMLElement>('[data-day]')].map(
      (day) => day.dataset.day,
    )
    expect(week).toEqual([
      '2026-09-14',
      '2026-09-15',
      '2026-09-16',
      '2026-09-17',
      '2026-09-18',
      '2026-09-19',
      '2026-09-20',
    ])

    fireEvent.click(within(main).getByRole('radio', { name: en.calendar.view.agenda }))
    await settled(client)
    // Today and the 22nd, which holds the audio, are days of their own; the
    // rest of September folds into the lines between them.
    const days = [...container.querySelectorAll<HTMLElement>('[data-day]')].map(
      (day) => day.dataset.day,
    )
    expect(days).toEqual(['2026-09-15', '2026-09-22'])
    expect(within(main).getByText(/14 days with nothing planned/)).toBeInTheDocument()
  })

  it('books a work from the pane on the day it is carried to', async () => {
    const { client, container } = renderApp('/calendar')
    await settled(client)
    const main = await screen.findByRole('main')
    fireEvent.click(within(main).getByRole('radio', { name: /^Works/ }))
    const row = await within(main).findByText('Paper Lanterns (short)')

    const day = container.querySelector('[data-day="2026-09-25"]')
    Object.defineProperty(document, 'elementFromPoint', { value: () => day, configurable: true })
    fireEvent.pointerDown(row, { button: 0, clientX: 10, clientY: 10 })
    await act(async () => {
      fireEvent.pointerMove(window, { clientX: 60, clientY: 60 })
      fireEvent.pointerUp(window, { clientX: 60, clientY: 60 })
    })
    await settled(client)

    // A release of its first door, already holding the day.
    expect(backend.argsOf('create_release')).toEqual([
      { release: { work_id: IDS.short, kind: 'short', scheduled_at: '2026-09-25' } },
    ])
  })

  it('shows a shift on the month before it books exactly what it showed', async () => {
    const plan = {
      moves: [{ release_id: IDS.audioRelease, from: '2026-09-22', to: '2026-09-29' }],
      kept: [],
      closed: [],
    } satisfies ShiftPlan
    backend.answer('plan_shift', () => plan)
    backend.answer('apply_shift', () => 1)
    const { client } = renderApp('/calendar')
    await settled(client)
    const main = await screen.findByRole('main')

    fireEvent.click(within(main).getByRole('button', { name: en.calendar.more }))
    fireEvent.click(await screen.findByRole('menuitem', { name: en.shift.menu }))
    const dialog = await screen.findByRole('dialog', { name: en.shift.title })
    fireEvent.click(within(dialog).getByRole('button', { name: en.shift.preview }))
    await settled(client)

    expect(backend.argsOf('plan_shift')).toEqual([
      { from: '2026-09-15', days: 7, today: '2026-09-15' },
    ])
    // Nothing moved yet: the bar asks, and the release is a ghost on the 29th.
    expect(backend.argsOf('apply_shift')).toEqual([])
    expect(await within(main).findByText(/1 release from .* would move 7 days later/)).toBeVisible()

    fireEvent.click(within(main).getByRole('button', { name: en.shift.apply }))
    await settled(client)
    expect(backend.argsOf('apply_shift')).toEqual([{ moves: plan.moves }])
  })

  it('keeps every date of the month on screen in one gesture', async () => {
    backend.answer('pin_range', () => 1)
    const { client } = renderApp('/calendar')
    await settled(client)
    const main = await screen.findByRole('main')

    fireEvent.click(within(main).getByRole('button', { name: en.calendar.more }))
    fireEvent.click(await screen.findByRole('menuitem', { name: en.calendar.pinMonth }))
    await settled(client)

    // The month's own days, never the neighbours that fill its weeks.
    expect(backend.argsOf('pin_range')).toEqual([
      { from: '2026-09-01', to: '2026-09-30', pinned: true },
    ])
  })

  it('draws a plan read from a CSV file as the layout does, says what was left out, and books it', async () => {
    backend.answer('plugin:dialog|open', () => 'C:/plans/november.csv')
    backend.answer('read_plan_import', () => ({
      placements: [{ release_id: IDS.audioRelease, date: '2026-10-06' }],
      problems: [{ line: 3, text: '2026-10-07,Nobody,youtube', reason: 'noWork' }],
    }))
    backend.answer('apply_layout', () => 1)
    const { client } = renderApp('/calendar')
    await settled(client)
    const main = await screen.findByRole('main')

    fireEvent.click(within(main).getByRole('button', { name: en.calendar.more }))
    fireEvent.click(await screen.findByRole('menuitem', { name: en.calendar.files.importPlan }))

    expect(await within(main).findByText(/1 release would land/)).toBeVisible()
    expect(backend.argsOf('read_plan_import')).toEqual([{ path: 'C:/plans/november.csv' }])
    expect(within(main).getByText(en.calendar.files.reason.noWork)).toBeInTheDocument()
    expect(backend.argsOf('apply_layout'), 'nothing is booked yet').toEqual([])

    fireEvent.click(within(main).getByRole('button', { name: en.calendar.layoutApply }))
    await settled(client)
    expect(backend.argsOf('apply_layout')).toEqual([
      { placements: [{ release_id: IDS.audioRelease, date: '2026-10-06' }] },
    ])
  })
})
