import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, within } from '@testing-library/react'
import type { ProfileConfig } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { openProfilePart } from '@/test/profile'
import { renderApp, settled } from '@/test/render'
import { answersFor, NOW, studio, type Studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The profile editor edits the whole document (v0.112): a list is added to
 * under a key, what would not save is said under the row it is about and
 * stops the Save, and a field can be a list.
 */

let backend: Backend
let workspace: Studio
/** What the backend says is wrong with the draft it is asked about. */
let wrong: (config: ProfileConfig) => unknown[]

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  workspace = studio()
  // One kind and two actions: the editor sizes every box it draws, and what
  // is tested here is the lists, not the size of the profile.
  const config = workspace.profile.config
  config.work_kinds = config.work_kinds.slice(0, 1)
  config.prompts = config.prompts.slice(0, 2)
  wrong = () => []
  backend = mockBackend({
    ...answersFor(workspace),
    check_profile_config: ({ config }) => wrong(config as ProfileConfig),
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

const bar = () =>
  document.querySelector<HTMLElement>(`[role="toolbar"][aria-label="${en.settings.unsaved}"]`)

/** The box a new entry's key is typed in, by the name of its list. */
function keyBox(list: string): HTMLInputElement {
  const label = en.editor.keyOf.replace('{{list}}', list)
  const found = document.querySelector<HTMLInputElement>(`input[aria-label="${label}"]`)
  expect(found, `no key box for ${list}`).not.toBeNull()
  return found!
}

async function open(part: string) {
  const view = renderApp('/settings/profile')
  await settled(view.client)
  await openProfilePart(part)
  return view
}

async function saved(client: Awaited<ReturnType<typeof open>>['client']) {
  fireEvent.click(within(bar()!).getByRole('button', { name: en.editor.save }))
  await settled(client)
  const [last] = backend.argsOf('update_profile_config').slice(-1)
  return last!.config as ProfileConfig
}

describe('the whole profile in the editor', () => {
  it('adds an axis under a key, as a scale of ten at the weight of one', async () => {
    const { client } = await open('Song')
    const before = workspace.profile.config.work_kinds[0]!.axes!.length

    const box = keyBox(en.editor.axes)
    fireEvent.change(box, { target: { value: 'Pull' } })
    fireEvent.click(screen.getByRole('button', { name: en.editor.addAxis }))

    const config = await saved(client)
    const axes = config.work_kinds[0]!.axes!
    expect(axes).toHaveLength(before + 1)
    // The key is the slug of what was typed; the word starts as the key.
    expect(axes.at(-1)).toEqual({ key: 'pull', label: 'pull', weight: 1, scale: 10, kind: 'scale' })
  })

  it('saves the draft on Ctrl+S, from the box being typed in', async () => {
    const { client } = await open('Song')
    const before = workspace.profile.config.work_kinds[0]!.axes!.length
    const box = keyBox(en.editor.axes)
    fireEvent.change(box, { target: { value: 'Pull' } })
    fireEvent.click(screen.getByRole('button', { name: en.editor.addAxis }))
    expect(bar()).not.toBeNull()

    fireEvent.keyDown(box, { key: 's', code: 'KeyS', ctrlKey: true })
    await settled(client)
    const [last] = backend.argsOf('update_profile_config').slice(-1)
    expect((last!.config as ProfileConfig).work_kinds[0]!.axes).toHaveLength(before + 1)
  })

  it('does not add an entry under a key that is taken or misshapen', async () => {
    await open('Song')
    const taken = workspace.profile.config.work_kinds[0]!.axes![0]!.key
    const add = screen.getByRole('button', { name: en.editor.addAxis })

    fireEvent.change(keyBox(en.editor.axes), { target: { value: taken } })
    expect(add).toBeDisabled()
    expect(screen.getByText(en.editor.keyTaken)).toBeVisible()

    fireEvent.change(keyBox(en.editor.axes), { target: { value: '9 lives' } })
    expect(add).toBeDisabled()
    expect(screen.getByText(en.editor.keyShape)).toBeVisible()
  })

  it('says what would not save under its row, and holds the Save until it is put right', async () => {
    const kind = workspace.profile.config.work_kinds[0]!
    const axis = kind.axes![0]!
    // The backend's own answer for an axis with no scale, as it travels.
    wrong = (config) =>
      config.work_kinds[0]!.axes![0]!.weight === 99
        ? [
            {
              key: 'problem.axisScale',
              params: {
                scale: '0',
                place: {
                  key: 'place.axis',
                  params: {
                    n: 1,
                    key: axis.key,
                    in: { key: 'place.workKind', params: { n: 1, key: kind.key } },
                  },
                },
              },
            },
          ]
        : []
    const { client } = await open('Song')

    const weight = document.querySelector<HTMLInputElement>(
      `input[aria-label="${axis.key} weight"]`,
    )!
    fireEvent.change(weight, { target: { value: '99' } })
    fireEvent.blur(weight)

    // Said in the window's words, with its place, under the axis.
    const said = await screen.findByText(
      `work kind 1 (\`${kind.key}\`): axis 1 (\`${axis.key}\`) has the scale 0; it must be above zero`,
    )
    expect(said.closest('li')).toHaveAttribute('data-reorder-id', axis.key)
    // A button of a toolbar stays focusable while it is off, and says so.
    await vi.waitFor(() =>
      expect(within(bar()!).getByRole('button', { name: en.editor.save })).toHaveAttribute(
        'aria-disabled',
        'true',
      ),
    )
    // The part it is in carries the mark.
    const parts = screen.getByRole('group', { name: en.editor.parts })
    expect(within(parts).getByRole('img', { name: en.editor.partWrong })).toBeVisible()

    fireEvent.change(weight, { target: { value: '7' } })
    fireEvent.blur(weight)
    await vi.waitFor(() =>
      expect(within(bar()!).getByRole('button', { name: en.editor.save })).not.toHaveAttribute(
        'aria-disabled',
        'true',
      ),
    )
    expect((await saved(client)).work_kinds[0]!.axes![0]!.weight).toBe(7)
  })

  it('adds a field and makes it a list', async () => {
    const { client } = await open(en.editor.part.fields)

    fireEvent.change(keyBox(en.editor.metaFields), { target: { value: 'guests' } })
    fireEvent.click(screen.getByRole('button', { name: en.editor.addField }))

    const type = await screen.findByRole('combobox', {
      name: en.editor.metaTypeOf.replace('{{key}}', 'guests'),
    })
    fireEvent.click(type)
    // Pressed the way a pointer does: Base UI picks on the press.
    const list = await screen.findByRole('option', { name: en.editor.metaType.list })
    fireEvent.pointerDown(list, { pointerType: 'mouse' })
    fireEvent.click(list)

    const config = await saved(client)
    expect(config.work_meta_fields.at(-1)).toMatchObject({ key: 'guests', type: 'list' })
  })
})
