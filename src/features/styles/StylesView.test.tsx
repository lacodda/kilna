import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, screen, waitFor, within } from '@testing-library/react'
import { emit } from '@tauri-apps/api/event'
import type { Asset, NewAsset, NewStyleBrick, StyleBrick } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio, type Studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The style dictionary: a grid of cards, the open style as a panel beside
 * it, and a style made in one gesture from the pictures that bring it.
 *
 * Until v0.79 a style was edited in a dialog with a Save button, and made in
 * three steps - a dialog for its name, a save, a second opening to hang a
 * picture on it - while the hint promised a drop that did not exist.
 */

const MADE = 'b-made'

let backend: Backend
let workspace: Studio

function asset(id: string, brick: string, name: string): Asset {
  return {
    id,
    profile_id: IDS.profile,
    work_id: null,
    release_id: null,
    kind: 'attachment',
    path: `C:/ws/media/${id}.png`,
    label: null,
    original_name: name,
    style_brick_id: brick,
    note_id: null,
    canon_fact_id: null,
    created_at: NOW,
  }
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  workspace = studio()
  backend = mockBackend({
    ...answersFor(workspace),
    create_style_brick: ({ brick }) => {
      const made: StyleBrick = {
        ...(brick as NewStyleBrick),
        id: MADE,
        profile_id: IDS.profile,
        description: null,
        hint: null,
        status: 'draft',
        created_at: NOW,
        updated_at: NOW,
        reference_count: 0,
        label: null,
        family: null,
        when_to_use: null,
        colours: [],
        sample: null,
        set_key: null,
        origin: 'own',
      }
      workspace.bricks.push(made)
      return made
    },
    attach_asset: ({ source, asset: to }) =>
      asset('a-dropped', (to as NewAsset).style_brick_id ?? '', String(source)),
    paste_style_reference: ({ id, name }) => asset('a-pasted', String(id), String(name)),
    update_style_brick: ({ id, patch }) => {
      const brick = workspace.bricks.find((one) => one.id === id)!
      Object.assign(brick, patch, { updated_at: '2026-09-15T10:05:00Z' })
      return brick
    },
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

/** A picture on the clipboard, pasted with Ctrl+V anywhere in the window. */
function paste(file: File) {
  const event = new Event('paste', { bubbles: true, cancelable: true })
  Object.defineProperty(event, 'clipboardData', { value: { files: [file] } })
  window.dispatchEvent(event)
}

describe('the style dictionary', () => {
  it('opens a style beside the cards, and writes it as it is typed into', async () => {
    const { client } = renderApp('/styles')
    await settled(client)

    const card = await screen.findByRole('button', { name: /Dusk over water/ })
    await act(async () => fireEvent.click(card))
    await settled(client)

    // A panel of the screen, not a dialog over it: the cards stay in reach,
    // narrowed to a column beside it, and the open one says so.
    expect(screen.queryByRole('dialog')).toBeNull()
    expect(screen.getByRole('button', { name: /Dusk over water/ })).toHaveAttribute(
      'aria-current',
      'true',
    )

    const description = await screen.findByRole('textbox', { name: en.styles.description })
    fireEvent.change(description, { target: { value: 'Low sun, teal against amber.' } })
    // No Save button: the pause after the typing writes it.
    expect(screen.queryByRole('button', { name: en.dialog.save })).toBeNull()
    await waitFor(() =>
      expect(backend.argsOf('update_style_brick')).toEqual([
        { id: IDS.brick, patch: { description: 'Low sun, teal against amber.' } },
      ]),
    )
    expect(backend.unanswered).toEqual([])
  })

  it('makes a style of a pasted picture and opens it, named to be typed over', async () => {
    const { client } = renderApp('/styles')
    await settled(client)
    await screen.findByText('Dusk over water')

    await act(async () => {
      paste(new File([new Uint8Array([137, 80, 78, 71])], 'dusk.png', { type: 'image/png' }))
      await new Promise((resolve) => setTimeout(resolve, 20))
    })
    await settled(client)

    // Made in the profile's first type, since no type is picked.
    expect(backend.argsOf('create_style_brick')).toEqual([
      { brick: { type_key: 'image-style', name: en.styles.untitled } },
    ])
    expect(backend.argsOf('paste_style_reference')).toEqual([
      { id: MADE, bytes: [137, 80, 78, 71], name: 'dusk.png' },
    ])
    const name = await screen.findByRole('textbox', { name: en.styles.name })
    expect(name).toHaveValue(en.styles.untitled)
    expect(name).toHaveFocus()
    expect(backend.unanswered).toEqual([])
  })

  it('makes a style of the pictures dropped on the dashed card, and leaves the rest', async () => {
    const { client } = renderApp('/styles')
    await settled(client)
    const main = await screen.findByRole('main')
    await within(main).findByText(en.styles.dropHint)
    // The drop listener is subscribed a turn after the card mounts.
    await act(() => new Promise((resolve) => setTimeout(resolve, 20)))

    await act(async () => {
      await emit('tauri://drag-drop', {
        paths: ['C:\\refs\\dusk.png', 'C:\\refs\\notes.txt'],
        position: { x: 0, y: 0 },
      })
      await new Promise((resolve) => setTimeout(resolve, 20))
    })
    await settled(client)

    expect(backend.argsOf('create_style_brick')).toHaveLength(1)
    expect(backend.argsOf('attach_asset')).toEqual([
      { source: 'C:\\refs\\dusk.png', asset: { style_brick_id: MADE } },
    ])
    expect(await screen.findByText(en.styles.notPictures.replace('{{count}}', '1'))).toBeVisible()
    expect(await screen.findByRole('textbox', { name: en.styles.name })).toHaveValue(
      en.styles.untitled,
    )
    expect(backend.unanswered).toEqual([])
  })
})

/** A brick of the starter set, as the seeding leaves it. */
function fromTheSet(change: Partial<StyleBrick> = {}): StyleBrick {
  return {
    id: 'b-set',
    profile_id: IDS.profile,
    type_key: 'image-style',
    name: 'Woodcut',
    description: 'STYLE: woodcut relief print.',
    hint: null,
    status: 'ready',
    created_at: NOW,
    updated_at: NOW,
    reference_count: 0,
    label: { en: 'Woodcut', ru: 'Ксилография' },
    family: 'classic',
    when_to_use: 'Raw, earthy songs.',
    colours: ['#121114', '#E8DCC4'],
    sample: null,
    set_key: 'woodcut',
    origin: 'set',
    ...change,
  }
}

describe('the starter set in the dictionary (v0.87)', () => {
  it('says where a style came from, and puts a changed one back as the set has it', async () => {
    workspace.bricks.push(fromTheSet({ origin: 'changed', description: 'My own words.' }))
    backend.answer('restore_style_brick', ({ id }) => {
      const brick = workspace.bricks.find((one) => one.id === id)!
      Object.assign(brick, { origin: 'set', description: 'STYLE: woodcut relief print.' })
      return brick
    })
    const { client } = renderApp('/styles/b-set')
    await settled(client)

    expect(await screen.findAllByText(en.styles.origin.changed)).not.toHaveLength(0)
    await act(async () => fireEvent.click(screen.getByRole('button', { name: en.styles.restore })))
    await settled(client)
    expect(backend.argsOf('restore_style_brick')).toEqual([{ id: 'b-set' }])
    expect(await screen.findByRole('textbox', { name: en.styles.description })).toHaveValue(
      'STYLE: woodcut relief print.',
    )
    expect(screen.queryByRole('button', { name: en.styles.restore })).toBeNull()
    expect(backend.unanswered).toEqual([])
  })

  it('narrows to the styles of one origin', async () => {
    workspace.bricks.push(fromTheSet())
    const { client } = renderApp('/styles')
    await settled(client)
    await screen.findByText('Woodcut')

    await act(async () =>
      fireEvent.click(screen.getByRole('button', { name: en.styles.originFilter.own })),
    )
    expect(screen.queryByText('Woodcut')).toBeNull()
    expect(screen.getByText('Dusk over water')).toBeVisible()

    await act(async () =>
      fireEvent.click(screen.getByRole('button', { name: en.styles.originFilter.set })),
    )
    expect(screen.getByText('Woodcut')).toBeVisible()
    expect(screen.queryByText('Dusk over water')).toBeNull()
  })

  it('reads a retired type and makes nothing new of it', async () => {
    const { client } = renderApp('/styles')
    await settled(client)
    await screen.findByText('Dusk over water')

    await act(async () =>
      fireEvent.click(screen.getByRole('button', { name: /Frame composition/ })),
    )
    await settled(client)
    expect(
      await screen.findByText(en.styles.retiredTitle.replace('{{type}}', 'Frame composition')),
    ).toBeVisible()
    // No dashed card under it, and "New style" makes one in a living type.
    expect(screen.queryByText(en.styles.dropHint)).toBeNull()
    await act(async () => fireEvent.click(screen.getByRole('button', { name: en.styles.new })))
    await settled(client)
    expect(backend.argsOf('create_style_brick')).toEqual([
      { brick: { type_key: 'image-style', name: en.styles.untitled } },
    ])
  })

  it('draws a background by its colour and keeps the colour typed until it is one', async () => {
    workspace.bricks.push(
      fromTheSet({
        id: 'b-bg',
        type_key: 'background',
        name: 'Paper',
        label: { en: 'Paper', ru: 'Бумага' },
        family: null,
        colours: ['#EFEBE3'],
        set_key: 'background-paper',
      }),
    )
    const { client } = renderApp('/styles/b-bg')
    await settled(client)

    const hex = await screen.findByPlaceholderText('#RRGGBB')
    expect(hex).toHaveValue('#EFEBE3')
    fireEvent.change(hex, { target: { value: '#1e9' } })
    await act(() => new Promise((resolve) => setTimeout(resolve, 700)))
    expect(backend.argsOf('update_style_brick')).toEqual([])
    fireEvent.change(hex, { target: { value: '#1e9e95' } })
    await waitFor(() =>
      expect(backend.argsOf('update_style_brick')).toEqual([
        { id: 'b-bg', patch: { colours: ['#1E9E95'] } },
      ]),
    )
  })
})
