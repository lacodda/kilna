import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, screen, within } from '@testing-library/react'
import type { BulkOutcome } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The collections, through every door that changes what one holds: the
 * collection's own page, the catalogue's bar, a row carried onto the shelf,
 * and a work's header. Each is held to the one write it should make - the
 * whole order, or the works to add - against the mocked backend.
 */

const added: BulkOutcome = { changed: 1, skipped: [] }
let backend: Backend

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  const workspace = studio()
  // Two works in the album, so it has an order to change.
  const harbour = workspace.works.find((work) => work.id === IDS.draft)!
  harbour.collection_id = IDS.collection
  workspace.collections[0]!.work_ids = [IDS.song, IDS.draft]
  backend = mockBackend({
    ...answersFor(workspace),
    set_collection_contents: () => null,
    add_to_collection: () => added,
  })
})

afterEach(() => {
  vi.useRealTimers()
  Reflect.deleteProperty(document, 'elementFromPoint')
})

describe('the collections screen', () => {
  it('draws each collection with how full it is against its goal', async () => {
    const { client } = renderApp('/collections')
    const main = await screen.findByRole('main')
    const card = (await within(main).findByText('Riverside')).closest('button')!
    await settled(client)

    expect(within(card).getByText('2 of 8')).toBeInTheDocument()
    expect(within(card).getByText('6 to go')).toBeInTheDocument()
  })

  it('puts the works in order from the keyboard, as one write of the whole list', async () => {
    const { client } = renderApp(`/collections/${IDS.collection}`)
    const row = await screen.findByRole('listitem', { name: 'Harbour Lights' })
    await settled(client)

    fireEvent.keyDown(row, { key: 'ArrowUp', altKey: true })
    await settled(client)

    expect(backend.argsOf('set_collection_contents')).toEqual([
      { id: IDS.collection, workIds: [IDS.draft, IDS.song] },
    ])
  })

  it('takes a work out by writing the list without it', async () => {
    const { client } = renderApp(`/collections/${IDS.collection}`)
    const out = await screen.findByRole('button', {
      name: en.collections.takeOutWork.replace('{{title}}', 'Paper Lanterns'),
    })
    await settled(client)

    fireEvent.click(out)
    await settled(client)

    expect(backend.argsOf('set_collection_contents')).toEqual([
      { id: IDS.collection, workIds: [IDS.draft] },
    ])
  })
})

describe('putting works in a collection from the catalogue', () => {
  it('sends the ticked works to the collection chosen in the bar', async () => {
    const { client } = renderApp('/catalogue')
    const tick = await screen.findByRole('checkbox', {
      name: en.catalogue.select.replace('{{title}}', 'Paper Lanterns (clip)'),
    })
    await settled(client)

    fireEvent.click(tick)
    fireEvent.click(await screen.findByRole('button', { name: en.catalogue.bulk.toCollection }))
    fireEvent.click(await screen.findByRole('menuitemcheckbox', { name: /Riverside/ }))
    await settled(client)

    expect(backend.argsOf('add_to_collection')).toEqual([
      { id: IDS.collection, workIds: [IDS.video] },
    ])
  })

  it('takes a row carried onto a collection on the shelf', async () => {
    const { client, container } = renderApp('/catalogue')
    const main = await screen.findByRole('main')
    const title = await within(main).findByText('Paper Lanterns (clip)', { exact: true })
    await settled(client)

    fireEvent.pointerDown(title, { button: 0, clientX: 10, clientY: 10 })
    await act(async () => {
      fireEvent.pointerMove(window, { clientX: 60, clientY: 60 })
    })
    // The shelf stands while the row is in the air; jsdom lays nothing out,
    // so where the pointer is let go is said.
    const shelf = container.ownerDocument.querySelector(
      `[data-collection-drop="${IDS.collection}"]`,
    )
    expect(shelf, 'the shelf offers the collection').not.toBeNull()
    Object.defineProperty(document, 'elementFromPoint', { value: () => shelf, configurable: true })
    await act(async () => {
      fireEvent.pointerUp(window, { clientX: 60, clientY: 60 })
    })
    await settled(client)

    expect(backend.argsOf('add_to_collection')).toEqual([
      { id: IDS.collection, workIds: [IDS.video] },
    ])
  })
})

describe("a work's header", () => {
  it('puts the work in a collection from the chip that names none', async () => {
    const { client } = renderApp(`/works/${IDS.video}/overview`)
    const chip = await screen.findByRole('button', { name: new RegExp(en.collections.pickNone) })
    await settled(client)

    fireEvent.click(chip)
    fireEvent.click(await screen.findByRole('menuitemcheckbox', { name: /Riverside/ }))
    await settled(client)

    expect(backend.argsOf('add_to_collection')).toEqual([
      { id: IDS.collection, workIds: [IDS.video] },
    ])
  })
})
