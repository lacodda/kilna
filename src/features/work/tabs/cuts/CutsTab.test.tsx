import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, waitFor } from '@testing-library/react'
import { emit } from '@tauri-apps/api/event'
import type { Cut, DonorParts, Plugin } from '@/lib/api/types'
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
    // The first screen of a file is the slow one: the whole app loads cold,
    // which the setup's wait ceiling allows for.
    expect(await screen.findByRole('button', { name: 'Paper Lanterns (short)' })).toBeVisible()
    expect(screen.getByText(en.cuts.fromThis)).toBeVisible()
    await settled(client)
    expect(backend.argsOf('list_cuts_from')).toContainEqual({ sourceId: IDS.video })
  })
})

describe('the splice', () => {
  it('puts a stretch in order from its fields, with Alt and an arrow', async () => {
    const { client } = renderApp(`/works/${IDS.short}/cuts`)
    const [start] = await screen.findAllByRole('textbox', { name: en.scenes.startsAt })
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
    await screen.findAllByRole('textbox', { name: en.scenes.startsAt })
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

  it('lands an end let go near where a part begins on it, unless Alt is held', async () => {
    backend.answer('cut_parts', () => [heard({ boundaries: [0, 19, 100] })])
    const { client, container } = renderApp(`/works/${IDS.short}/cuts`)
    await screen.findAllByRole('textbox', { name: en.scenes.startsAt })
    await settled(client)
    await waitFor(() => expect(container.querySelector('[data-marks] span')).not.toBeNull())
    const named = container.querySelectorAll('[title="Chorus · 0:12 – 0:30"]')
    const hand = named[named.length - 1]!

    // Six seconds on: the start would be at 18, a second short of a part.
    fireEvent.pointerDown(hand, { button: 0, clientX: 20, pointerId: 1 })
    fireEvent.pointerMove(hand, { clientX: 26, pointerId: 1 })
    fireEvent.pointerUp(hand, { clientX: 26, pointerId: 1 })
    await waitFor(() =>
      expect(backend.argsOf('update_cut')).toEqual([
        { id: IDS.cut, patch: { starts_at: 19, ends_at: 37 } },
      ]),
    )
  })

  it('leaves an end where Alt let it go', async () => {
    backend.answer('cut_parts', () => [heard({ boundaries: [0, 19, 100] })])
    const { client, container } = renderApp(`/works/${IDS.short}/cuts`)
    await screen.findAllByRole('textbox', { name: en.scenes.startsAt })
    await settled(client)
    await waitFor(() => expect(container.querySelector('[data-marks] span')).not.toBeNull())
    const named = container.querySelectorAll('[title="Chorus · 0:12 – 0:30"]')
    const hand = named[named.length - 1]!

    fireEvent.pointerDown(hand, { button: 0, clientX: 20, pointerId: 1 })
    fireEvent.pointerMove(hand, { clientX: 26, pointerId: 1, altKey: true })
    fireEvent.pointerUp(hand, { clientX: 26, pointerId: 1, altKey: true })
    await waitFor(() =>
      expect(backend.argsOf('update_cut')).toEqual([
        { id: IDS.cut, patch: { starts_at: 18, ends_at: 36 } },
      ]),
    )
  })
})

/** What was heard in the short's clip, and what else is offered for it. */
function heard(over: Partial<DonorParts> = {}): DonorParts {
  return {
    source_id: IDS.video,
    source_title: 'Paper Lanterns (video)',
    duration: 204,
    hearing: 'heard',
    boundaries: [0, 60, 90, 100, 124],
    parts: [],
    taken: [],
    ...over,
  }
}

describe('where to cut from', () => {
  it('offers the parts heard and placed by the words, and takes one as a stretch', async () => {
    backend.answer('cut_parts', () => [
      heard({
        parts: [
          { starts_at: 60, ends_at: 90, reason: 'chorus', detail: null, source: 'sound' },
          {
            starts_at: 100,
            ends_at: 124,
            reason: 'wordsChorus',
            detail: 'Chorus 2: Hold the lantern',
            source: 'words',
          },
        ],
        taken: [
          { work_id: 'w-sibling', title: 'Paper Lanterns (short 2)', starts_at: 98, ends_at: 126 },
        ],
      }),
    ])
    backend.answer('create_cut', ({ cut }) => ({
      ...workspace.cuts[0]!,
      ...(cut as object),
      id: 'cut-new',
    }))
    const { client, container } = renderApp(`/works/${IDS.short}/cuts`)

    expect(await screen.findByText(en.cuts.parts.reason.chorus)).toBeVisible()
    expect(screen.getByText(en.cuts.parts.reason.wordsChorus)).toBeVisible()
    expect(
      screen.getByText(en.cuts.parts.alreadyIn.replace('{{title}}', 'Paper Lanterns (short 2)')),
      'a part that lies in what a sibling took says so',
    ).toBeVisible()
    expect(container.querySelector('[data-lane]'), 'drawn under the track').not.toBeNull()
    await settled(client)

    fireEvent.click(
      screen.getByRole('button', {
        name: en.cuts.parts.takeThis.replace('{{span}}', '1:00 – 1:30'),
      }),
    )

    await waitFor(() =>
      expect(backend.argsOf('create_cut')).toEqual([
        { cut: { work_id: IDS.short, source_id: IDS.video, starts_at: 60, ends_at: 90 } },
      ]),
    )
  })

  it('says a video nobody listened to yet, and one rendered again since', async () => {
    backend.answer('cut_parts', () => [heard({ hearing: 'unheard', boundaries: [] })])
    renderApp(`/works/${IDS.short}/cuts`)
    expect(await screen.findByText(en.cuts.parts.unheard)).toBeVisible()
  })
})

describe('a cutter', () => {
  /** A plugin whose one command cuts a short, on this tab. */
  const CUTTER: Plugin = {
    executable: 'kilna-plugin-cut',
    path: 'C:/plugins/kilna-plugin-cut.exe',
    usable: true,
    reason: null,
    manifest: {
      protocol_version: 1,
      name: 'cut',
      version: '0.1.0',
      description: null,
      commands: [
        { key: 'cut', label: 'Cut the short', description: null, target: 'cuts', timeout: 900 },
        { key: 'count', label: 'Count words', description: null, target: 'work' },
      ],
    },
  }

  it('stands on the tab, says how far it has got, and stops when asked', async () => {
    let fail: (reason: unknown) => void = () => {}
    backend.answer('list_plugins', () => [CUTTER])
    backend.answer(
      'run_plugin',
      () =>
        new Promise((_, reject) => {
          fail = reject
        }),
    )
    backend.answer('cancel_plugin', () => {
      fail({ kind: 'refused', code: 'plugin.cancelled', message: 'The plugin was stopped.' })
      return null
    })
    const { client } = renderApp(`/works/${IDS.short}/cuts`)
    const cut = await screen.findByRole('button', { name: 'Cut the short' })
    expect(
      screen.queryByRole('button', { name: 'Count words' }),
      "a command of a work's head is not a cutter",
    ).toBeNull()
    await settled(client)

    fireEvent.click(cut)
    await waitFor(() => expect(backend.argsOf('run_plugin')).toHaveLength(1))
    const sent = backend.argsOf('run_plugin')[0]!
    expect(sent).toMatchObject({
      executable: 'kilna-plugin-cut',
      command: 'cut',
      target: 'cuts',
      id: IDS.short,
    })

    // Another run's progress is not this one's.
    await emit('plugin-progress', { run: 'someone-else', progress: 0.9, message: 'elsewhere' })
    await emit('plugin-progress', { run: sent.run, progress: 0.4, message: 'cutting 2 of 3' })
    expect(await screen.findByText('cutting 2 of 3')).toBeVisible()
    expect(screen.queryByText('elsewhere')).toBeNull()
    expect(screen.getByRole('progressbar')).toBeVisible()

    fireEvent.click(screen.getByRole('button', { name: en.plugins.stop }))
    await waitFor(() => expect(backend.argsOf('cancel_plugin')).toEqual([{ run: sent.run }]))
    expect(await screen.findByRole('button', { name: 'Cut the short' })).toBeVisible()
    expect(await screen.findByText(en.plugins.stopped)).toBeVisible()
  })

  it('names the file a finished cut made the short’s finished file', async () => {
    backend.answer('list_plugins', () => [CUTTER])
    backend.answer('run_plugin', () => ({
      message: null,
      files: ['C:/logs/cut.log', 'C:/media/shorts/v2/video/v2.mp4'],
      finished: 'C:/media/shorts/v2/video/v2.mp4',
    }))
    const { client } = renderApp(`/works/${IDS.short}/cuts`)
    const cut = await screen.findByRole('button', { name: 'Cut the short' })
    await settled(client)

    fireEvent.click(cut)

    expect(
      await screen.findByText(en.plugins.madeFinished.replace('{{name}}', 'v2.mp4')),
      'the finished file, not the first file named',
    ).toBeVisible()
  })
})
