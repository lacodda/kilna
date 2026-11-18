import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, waitFor, within } from '@testing-library/react'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, NOW, studio, type Studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The first start of a new machine (v0.129): the window welcomes the person
 * into the space kilna made - Claude Code, what the space is made of with a
 * demo or not, then the first work by hand.
 */

let backend: Backend
let workspace: Studio

function welcomed(found: boolean) {
  workspace = studio()
  const answers = answersFor(workspace)
  const current = answers.current_space!({}) as { space: Record<string, unknown> }
  backend = mockBackend({
    ...answers,
    current_space: () => ({ ...current, space: { ...current.space, welcome: true } }),
    assistant_status: () => ({
      available: found,
      version: found ? '2.4.0' : null,
      reason: found ? null : 'not found',
    }),
    welcome_space: () => ({ ...current, space: { ...current.space, welcome: false } }),
    seed_demo: () => ({ works: 7, notes: 4, releases: 2 }),
    create_work: ({ work }) => ({
      ...workspace.works[0],
      id: 'w-first',
      title: (work as { title: string }).title,
    }),
  })
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

async function welcome() {
  const view = renderApp('/dashboard')
  await settled(view.client)
  return screen.findByRole('dialog', { name: en.welcome.check.title })
}

describe('the welcome', () => {
  it('says whether Claude Code is here, and how to get it when it is not', async () => {
    welcomed(false)
    const dialog = await welcome()
    expect(await within(dialog).findByText(en.welcome.check.install)).toBeVisible()
  })

  it('makes the space as chosen, with a demo, and stops welcoming', async () => {
    welcomed(true)
    const dialog = await welcome()
    expect(await within(dialog).findByText(/Claude Code is here \(2\.4\.0\)/)).toBeVisible()
    fireEvent.click(within(dialog).getByRole('button', { name: en.welcome.next }))

    const shape = await screen.findByRole('dialog', { name: en.welcome.space.title })
    fireEvent.click(within(shape).getByRole('button', { name: en.spaces.kinds.blog }))
    fireEvent.change(within(shape).getByRole('textbox', { name: en.spaces.name }), {
      target: { value: 'Field Notes' },
    })
    fireEvent.click(within(shape).getByRole('checkbox', { name: en.welcome.space.demo }))
    fireEvent.click(within(shape).getByRole('button', { name: en.welcome.space.make }))

    await waitFor(() => expect(backend.argsOf('seed_demo')).toHaveLength(1))
    expect(backend.argsOf('welcome_space')[0]).toMatchObject({
      choice: { name: 'Field Notes', kind: 'blog' },
    })
    await waitFor(() =>
      expect(screen.queryByRole('dialog', { name: en.welcome.space.title })).toBeNull(),
    )
  })

  it('makes the first work by hand when there is no demo', async () => {
    welcomed(true)
    const dialog = await welcome()
    fireEvent.click(within(dialog).getByRole('button', { name: en.welcome.next }))
    const shape = await screen.findByRole('dialog', { name: en.welcome.space.title })
    fireEvent.click(within(shape).getByRole('button', { name: en.welcome.space.make }))

    const first = await screen.findByRole('dialog', { name: en.welcome.first.title })
    fireEvent.change(within(first).getByRole('textbox', { name: en.welcome.first.titleLabel }), {
      target: { value: 'Harbour lights' },
    })
    fireEvent.click(within(first).getByRole('button', { name: en.welcome.first.make }))
    await waitFor(() =>
      expect(backend.argsOf('create_work')[0]).toMatchObject({
        work: { title: 'Harbour lights' },
      }),
    )
    expect(backend.argsOf('seed_demo')).toEqual([])
  })
})
