import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, within } from '@testing-library/react'
import type { ProfileConfig } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, NOW, studio, type Studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The guard of repeats' three numbers in the profile editor (v0.90, ADR
 * 0054): shown at kilna's defaults while the profile names none, and written
 * into the profile's document through its draft and the bar's Save, as every
 * other part of the profile is.
 */

let backend: Backend
let workspace: Studio

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  workspace = studio()
  // One kind and one action: the editor sizes every box it draws, and what
  // is tested here is three numbers, not the size of the profile.
  const config = workspace.profile.config
  config.work_kinds = config.work_kinds.slice(0, 1)
  config.prompts = config.prompts.slice(0, 1)
  backend = mockBackend({
    ...answersFor(workspace),
    update_profile_config: ({ config }) => {
      workspace.profile = { ...workspace.profile, config: config as ProfileConfig }
      return workspace.profile
    },
  })
})

afterEach(() => {
  vi.useRealTimers()
})

/** A number of the guard, by its name: role queries over the whole editor
 *  take seconds in jsdom. */
function box(label: string): HTMLInputElement {
  const found = document.querySelector<HTMLInputElement>(`input[aria-label="${label}"]`)
  expect(found, `no box for ${label}`).not.toBeNull()
  return found!
}

const bar = () =>
  document.querySelector<HTMLElement>(`[role="toolbar"][aria-label="${en.settings.unsaved}"]`)

describe("the guard's numbers", () => {
  it('shows the defaults while the profile names no guard', async () => {
    const { client } = renderApp('/settings/profile')
    await settled(client)
    await vi.waitFor(() => box(en.editor.guardWindow))

    expect(box(en.editor.guardWindow)).toHaveValue('90')
    expect(box(en.editor.guardRank).value.replace(/\D/g, '')).toBe('20000')
    expect(box(en.editor.guardWorks)).toHaveValue('2')
  })

  it('writes the whole guard into the profile when one number moves', async () => {
    const { client } = renderApp('/settings/profile')
    await settled(client)
    await vi.waitFor(() => box(en.editor.guardWindow))

    fireEvent.change(box(en.editor.guardWindow), { target: { value: '60' } })
    fireEvent.blur(box(en.editor.guardWindow))
    await vi.waitFor(() => expect(bar()).not.toBeNull())

    await act(async () => {
      fireEvent.click(within(bar()!).getByRole('button', { name: en.editor.save }))
    })
    await settled(client)

    const [saved] = backend.argsOf('update_profile_config')
    expect((saved!.config as ProfileConfig).guard).toEqual({
      window_days: 60,
      rare_rank: 20_000,
      rare_in_works: 2,
    })
  })
})
