import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, within } from '@testing-library/react'
import type { SpaceType } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * Types of space (v0.112, ADR 0079): a profile kept under a name, listed in
 * Settings → Spaces, offered beside the shipped kinds when a space is made,
 * and made from the open space's profile in Settings → Profile.
 */

const NIGHT: SpaceType = {
  id: 'type-1',
  name: 'Night Shift',
  description: null,
  kind: 'music',
  work_kinds: 5,
  created_at: '2026-11-10T08:00:00.000Z',
}

let backend: Backend
let types: SpaceType[]

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  types = [NIGHT]
  const workspace = studio()
  // The profile section is opened by one test: a small profile keeps it quick.
  workspace.profile.config.work_kinds = workspace.profile.config.work_kinds.slice(0, 1)
  workspace.profile.config.prompts = workspace.profile.config.prompts.slice(0, 1)
  backend = mockBackend({
    ...answersFor(workspace),
    list_space_types: () => types,
    delete_space_type: ({ id }) => {
      types = types.filter((type) => type.id !== id)
      return null
    },
    save_space_type: ({ name }) => {
      const kept = { ...NIGHT, id: 'type-2', name: name as string }
      types = [...types, kept]
      return kept
    },
    create_space: ({ space }) => ({
      id: 'space-2',
      name: (space as { name: string }).name,
      kind: 'music',
      colour: null,
      mark: null,
      path: 'C:/spaces/second',
      opened_at: null,
      trashed_at: null,
      ask_name: false,
      reachable: true,
      open: false,
    }),
    open_space: () => {
      throw new Error('not opened in this test')
    },
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

describe('types of space', () => {
  it('are listed with what they grew from, and let go from their row', async () => {
    const { client } = renderApp('/settings/spaces')
    await settled(client)

    expect(await screen.findByText('Night Shift')).toBeVisible()
    expect(screen.getByText(en.spaces.types)).toBeVisible()
    // Said by how many kinds of work it names, where it has no description.
    expect(screen.getByText(en.spaces.typeKinds_other.replace('{{count}}', '5'))).toBeVisible()

    fireEvent.click(
      screen.getByRole('button', {
        name: en.spaces.typeActions.replace('{{name}}', 'Night Shift'),
      }),
    )
    fireEvent.click(await screen.findByRole('menuitem', { name: en.spaces.deleteType }))
    await settled(client)

    expect(backend.argsOf('delete_space_type')).toEqual([{ id: 'type-1' }])
    expect(screen.queryByText('Night Shift')).toBeNull()
  })

  it('are offered when a space is made, and the space is made of the one chosen', async () => {
    const { client } = renderApp('/settings/spaces')
    await settled(client)

    fireEvent.click(screen.getByRole('button', { name: en.spaces.new }))
    const dialog = await screen.findByRole('dialog', { name: en.spaces.newTitle })
    // The shipped kinds, the empty one among them, and the type.
    expect(within(dialog).getByRole('button', { name: en.spaces.kinds.blank })).toBeVisible()
    fireEvent.click(within(dialog).getByRole('button', { name: 'Night Shift' }))
    expect(
      within(dialog).getByText(en.spaces.typeHint.replace('{{kind}}', en.spaces.kinds.music)),
    ).toBeVisible()

    fireEvent.change(within(dialog).getByRole('textbox', { name: en.spaces.name }), {
      target: { value: 'Second channel' },
    })
    fireEvent.click(within(dialog).getByRole('button', { name: en.spaces.create }))
    await settled(client)

    expect(backend.argsOf('create_space')).toEqual([
      {
        space: {
          name: 'Second channel',
          kind: 'music',
          colour: null,
          mark: null,
          type_id: 'type-1',
        },
      },
    ])
  })

  it('are kept from the open space’s profile, under a name', async () => {
    const { client } = renderApp('/settings/profile')
    await settled(client)

    fireEvent.click(await screen.findByRole('button', { name: en.editor.saveAsType }))
    const dialog = await screen.findByRole('dialog', { name: en.spaces.saveTypeTitle })
    const name = within(dialog).getByRole('textbox', { name: en.spaces.typeName })
    fireEvent.change(name, { target: { value: 'Late Shift' } })
    fireEvent.click(within(dialog).getByRole('button', { name: en.spaces.saveTypeConfirm }))
    await settled(client)

    expect(backend.argsOf('save_space_type')).toEqual([{ name: 'Late Shift' }])
    expect(
      await screen.findByText(en.spaces.typeSaved.replace('{{name}}', 'Late Shift')),
    ).toBeVisible()
  })
})
