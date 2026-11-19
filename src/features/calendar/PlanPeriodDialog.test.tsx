import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, within } from '@testing-library/react'
import type { PeriodLayout, PeriodPlan } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * A period laid out ahead (v0.152, FR-20 20.3): the days the rhythm gives
 * are shown before anything is made, a day off and a day already taken
 * passed over, and one press makes the month's collection and its drafts.
 */

let backend: Backend

const PLAN: PeriodPlan = {
  months: [
    {
      month: '2026-10',
      target: 3,
      slots: [
        { date: '2026-10-06', floating: false, closed: false, taken: null },
        { date: '2026-10-08', floating: false, closed: true, taken: null },
        { date: '2026-10-10', floating: false, closed: false, taken: 'Paper Lanterns' },
        { date: '2026-10-17', floating: true, closed: false, taken: null },
      ],
    },
  ],
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  backend = mockBackend({
    ...answersFor(studio()),
    plan_period: () => PLAN,
    lay_out_period: () => ({ collections: 1, works: 2 }),
  })
})

afterEach(() => {
  vi.useRealTimers()
})

describe('a period planned', () => {
  it('shows the days, passes over the closed and the taken, and makes the rest', async () => {
    const { client } = renderApp('/calendar')
    await settled(client)
    const main = await screen.findByRole('main')
    fireEvent.click(within(main).getByRole('button', { name: en.calendar.more }))
    fireEvent.click(await screen.findByRole('menuitem', { name: en.period.menu }))
    const dialog = await screen.findByRole('dialog', { name: en.period.title })
    await settled(client)

    // The month after today's, a month long, unless said otherwise.
    expect(backend.argsOf('plan_period')).toEqual([{ from: '2026-10', months: 1 }])
    expect(within(dialog).getByText('2 of 3')).toBeInTheDocument()

    fireEvent.click(within(dialog).getByRole('button', { name: 'Make 2 drafts' }))
    await settled(client)

    const [asked] = backend.argsOf('lay_out_period') as { layout: PeriodLayout }[]
    expect(asked?.layout.kind).toBe('song')
    expect(asked?.layout.collection_kind).toBe('album')
    expect(asked?.layout.months[0]?.drafts.map((draft) => draft.date)).toEqual([
      '2026-10-06',
      '2026-10-17',
    ])
    expect(asked?.layout.months[0]?.title).toMatch(/2026/)
  })
})
