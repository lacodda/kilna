import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, within } from '@testing-library/react'
import type { ProfileConfig } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, NOW, studio, type Studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The profile's draft outlives the section it is edited in.
 *
 * Until v0.79 the editor kept its edits in its own state, and leaving the
 * Profile section - for another section, another screen - threw them away
 * without a word. Here an edit is made, the section is left and come back to,
 * and the edit is where it was; the bar at the pane's foot says it is unsaved
 * for as long as it is, and the list says so from any other section.
 */

let backend: Backend
let workspace: Studio

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  workspace = studio()
  // One kind and two actions of the Studio rather than all of them: each
  // action's method and each release field is a box that sizes itself, jsdom
  // measures every one of them on every render, and what is tested here is
  // the draft, not the size of the profile.
  const config = workspace.profile.config
  config.work_kinds = config.work_kinds.slice(0, 1)
  config.prompts = config.prompts.slice(0, 2)
  backend = mockBackend({
    ...answersFor(workspace),
    // Stored as the backend stores it, so the refetch after a save reads it.
    update_profile_config: ({ config }) => {
      workspace.profile = { ...workspace.profile, config: config as ProfileConfig }
      return workspace.profile
    },
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

/** The first kind's first axis: its key and the box its label is typed in. */
function firstAxis() {
  const axis = workspace.profile.config.work_kinds[0]!.axes![0]!
  const box = () => {
    const found = document.querySelector<HTMLInputElement>(`input[aria-label="${axis.key} label"]`)
    expect(found, `no box for the ${axis.key} axis`).not.toBeNull()
    return found!
  }
  return { axis, box }
}

/** A section's row in the screen's own list, found by its address: role
 *  queries over the whole profile editor take seconds in jsdom. */
function section(key: string): HTMLElement {
  const link = document.querySelector<HTMLElement>(`main nav a[href="/settings/${key}"]`)
  expect(link, `no row for ${key}`).not.toBeNull()
  return link!
}

const bar = () =>
  document.querySelector<HTMLElement>(`[role="toolbar"][aria-label="${en.settings.unsaved}"]`)

async function openProfile() {
  const view = renderApp('/settings/profile')
  await settled(view.client)
  await vi.waitFor(() => firstAxis().box())
  return view
}

describe('the profile draft', () => {
  it('is kept when the section is left, and said to be unsaved from anywhere', async () => {
    await openProfile()
    const { box } = firstAxis()
    expect(bar(), 'a bar before anything was edited').toBeNull()

    fireEvent.change(box(), { target: { value: 'Stays with you' } })
    expect(bar()).not.toBeNull()

    fireEvent.click(section('general'))
    await screen.findByText(en.settings.appearance)
    expect(bar(), 'the bar belongs to the profile').toBeNull()
    // The row of the section with the draft says so.
    expect(within(section('profile')).getByRole('img')).toHaveAccessibleName(en.settings.unsaved)

    fireEvent.click(section('profile'))
    await vi.waitFor(() => box())
    expect(box()).toHaveValue('Stays with you')
    expect(bar()).not.toBeNull()
  })

  it('is thrown away by Discard, and put back by its undo', async () => {
    await openProfile()
    const { box } = firstAxis()
    const before = box().value

    fireEvent.change(box(), { target: { value: 'Stays with you' } })
    fireEvent.click(within(bar()!).getByRole('button', { name: en.settings.discard }))
    expect(box()).toHaveValue(before)
    expect(bar()).toBeNull()

    fireEvent.click(await screen.findByText(en.toast.undo))
    expect(box()).toHaveValue('Stays with you')
    expect(bar()).not.toBeNull()
  })

  it('is saved whole, and lets go once it is what is stored', async () => {
    const { client } = await openProfile()
    const { axis, box } = firstAxis()
    const prompts = structuredClone(workspace.profile.config.prompts)

    fireEvent.change(box(), { target: { value: 'Stays with you' } })
    fireEvent.click(within(bar()!).getByRole('button', { name: en.editor.save }))
    await settled(client)

    const [saved] = backend.argsOf('update_profile_config')
    const config = saved!.config as ProfileConfig
    expect(config.work_kinds[0]!.axes!.find((entry) => entry.key === axis.key)!.label).toBe(
      'Stays with you',
    )
    // The rest of the profile goes back as it was stored.
    expect(config.prompts).toEqual(prompts)

    expect(bar()).toBeNull()
    expect(box()).toHaveValue('Stays with you')
    expect(Object.keys(localStorage).filter((key) => key.startsWith('kilna.profileDraft'))).toEqual(
      [],
    )
  })
})
