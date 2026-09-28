import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, waitFor } from '@testing-library/react'
import type { Cut } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio, type Studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The Cuts tab: a donor's track with stretches that move by hand, the list
 * under it put in order by a grip, and - on the donor's own card - what was
 * cut out of it.
 */

/** A second stretch of the short, out of the same clip, after the first. */
const VERSE = 'cut-verse'

let backend: Backend
let workspace: Studio

beforeEach(() => {
  workspace = studio()
  const first = workspace.cuts[0]!
  const verse: Cut = {
    ...first,
    id: VERSE,
    starts_at: 40,
    ends_at: 48,
    position: 1,
    label: 'Verse',
    created_at: NOW,
  }
  workspace.cuts.push(verse)
  backend = mockBackend({
    ...answersFor(workspace),
    reorder_cuts: ({ ids }) => ids,
    update_cut: ({ id, patch }) => ({
      ...workspace.cuts.find((cut) => cut.id === id)!,
      ...(patch as Partial<Cut>),
    }),
  })
})

describe('the donor', () => {
  it('names the works cut out of it, on its own card', async () => {
    const { client } = renderApp(`/works/${IDS.video}/cuts`)

    // The short took its stretches from this clip; its name is read from
    // the work itself, since no link says it was made from the clip.
    // The first screen of a file is the slow one: the whole app loads cold.
    expect(
      await screen.findByRole('button', { name: 'Paper Lanterns (short)' }, { timeout: 5000 }),
    ).toBeVisible()
    expect(screen.getByText(en.cuts.fromThis)).toBeVisible()
    await settled(client)
    expect(backend.argsOf('list_cuts_from')).toContainEqual({ sourceId: IDS.video })
  })
})

describe('the splice', () => {
  it('puts a stretch in order from its fields, with Alt and an arrow', async () => {
    const { client } = renderApp(`/works/${IDS.short}/cuts`)
    const [start] = await screen.findAllByRole(
      'textbox',
      { name: en.scenes.startsAt },
      { timeout: 5000 },
    )
    await settled(client)

    fireEvent.keyDown(start!, { key: 'ArrowDown', altKey: true })

    await waitFor(() =>
      expect(backend.argsOf('reorder_cuts')).toEqual([
        { workId: IDS.short, ids: [VERSE, IDS.cut] },
      ]),
    )
  })
})

describe('the track', () => {
  // jsdom lays nothing out: the track is said to be 204 pixels wide, one per
  // second of the 204-second clip, so a pixel dragged is a second moved.
  const box = { x: 0, y: 0, top: 0, left: 0, bottom: 30, right: 204, width: 204, height: 30 }

  beforeEach(() => {
    vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockReturnValue({
      ...box,
      toJSON: () => box,
    })
    Element.prototype.setPointerCapture ??= () => {}
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('writes a dragged stretch once, where it was let go', async () => {
    const { client, container } = renderApp(`/works/${IDS.short}/cuts`)
    await screen.findAllByRole('textbox', { name: en.scenes.startsAt }, { timeout: 5000 })
    await settled(client)

    // The hand over the stretch, not the Track's own segment under it: the
    // last of the two elements that carry its name.
    const named = container.querySelectorAll('[title="Chorus · 0:12 – 0:30"]')
    const hand = named[named.length - 1]!

    fireEvent.pointerDown(hand, { button: 0, clientX: 20, pointerId: 1 })
    fireEvent.pointerMove(hand, { clientX: 23, pointerId: 1 })
    fireEvent.pointerMove(hand, { clientX: 26, pointerId: 1 })
    // Nothing is written while it moves.
    expect(backend.argsOf('update_cut')).toEqual([])
    fireEvent.pointerUp(hand, { clientX: 26, pointerId: 1 })

    await waitFor(() =>
      expect(backend.argsOf('update_cut')).toEqual([
        { id: IDS.cut, patch: { starts_at: 18, ends_at: 36 } },
      ]),
    )
  })
})
