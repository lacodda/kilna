import { beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, waitFor } from '@testing-library/react'
import { emit } from '@tauri-apps/api/event'
import type { Plugin, ShortView, WorkPatch } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, shortView, studio, type Studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

// The layers of a short are drawn on a canvas, which this window has not got:
// what is drawn is the layers' own test's to hold, and here only that they
// are laid before the builder runs.
vi.mock('@/lib/shorts/layers', () => ({
  draw: () => Promise.resolve({}),
  paint: () => undefined,
  pictureOf: () => Promise.reject(new Error('no pictures here')),
  layersOf: () =>
    Promise.resolve({
      placed: { ground: 'ground.png' },
      layers: [{ name: 'ground.png', bytes: [1, 2, 3] }],
    }),
}))

/*
 * The Build tab of a short (v0.168): the variant of the space's short style
 * it wears, what it is built from and what it lacks, and the build - the
 * layers laid, then kilna-plugin-cut run - with its progress.
 */

const BUILDER: Plugin = {
  executable: 'kilna-plugin-cut',
  path: 'C:/plugins/kilna-plugin-cut.exe',
  usable: true,
  reason: null,
  enabled: true,
  settings: {},
  manifest: {
    protocol_version: 1,
    name: 'cut',
    version: '0.4.0',
    description: null,
    descriptions: {},
    settings: [],
    commands: [
      {
        key: 'cut',
        label: 'Build the short',
        labels: {},
        description: null,
        descriptions: {},
        target: 'cuts',
        timeout: 1800,
      },
    ],
  },
}

let backend: Backend
let workspace: Studio
let view: ShortView
let written: WorkPatch[]

beforeEach(() => {
  workspace = studio()
  view = shortView()
  written = []
  backend = mockBackend({
    ...answersFor(workspace),
    list_plugins: () => [BUILDER],
    short_view: () => view,
    update_work: ({ id, patch }) => {
      written.push(patch as WorkPatch)
      const work = workspace.works.find((one) => one.id === id)!
      Object.assign(work, { meta: { ...work.meta, ...(patch as WorkPatch).meta } })
      return work
    },
    run_plugin: () => ({
      message: 'Built v1.mp4: 0:13',
      files: ['C:/media/shorts/s1/video/v1.mp4'],
      finished: 'C:/media/shorts/s1/video/v1.mp4',
      proposed_score: null,
      candidates: [],
      quota: null,
    }),
  })
})

/** Choose an answer the way a pointer does: the press starts on the item,
 *  which is what the picker asks of a mouse before it takes the click. */
function choose(option: HTMLElement) {
  fireEvent.pointerDown(option, { pointerType: 'mouse' })
  fireEvent.click(option)
}

const open = async () => {
  const app = renderApp(`/works/${IDS.short}/build`)
  await screen.findByText(en.shorts.madeOf)
  await settled(app.client)
  return app
}

describe("a short's Build tab", () => {
  it('says what the short is built from, what it lacks and which variant it wears', async () => {
    await open()

    expect(screen.getByText(en.shorts.missing.cover)).toBeVisible()
    expect(screen.getByText(en.shorts.words.noTimings)).toBeVisible()
    expect(
      screen.getByText(en.shorts.wornByCalendar.replace('{{name}}', 'Day'), { exact: false }),
    ).toBeVisible()
    expect(
      screen.getByRole('img', { name: en.shorts.frame.replace('{{title}}', 'PAPER LANTERNS') }),
    ).toBeVisible()
    // A cover is not what stops a build: the short is built without its tail.
    expect(screen.getByRole('button', { name: en.shorts.build })).toBeEnabled()
  })

  it('does not build a short that lacks what a build cannot start without', async () => {
    view = {
      ...view,
      missing: [{ what: 'source', title: 'Paper Lanterns (clip)' }, { what: 'cover' }],
    }
    await open()

    expect(
      screen.getByText(en.shorts.missing.source.replace('{{title}}', 'Paper Lanterns (clip)')),
    ).toBeVisible()
    expect(screen.getByRole('button', { name: en.shorts.build })).toBeDisabled()
  })

  it('says the builder is not installed rather than offering a build', async () => {
    backend.answer('list_plugins', () => [])
    await open()

    expect(screen.getByText(en.shorts.noBuilder)).toBeVisible()
    expect(screen.getByRole('button', { name: en.shorts.build })).toBeDisabled()
  })

  it('lays the layers, runs the builder, shows how far it is and names the file', async () => {
    await open()

    fireEvent.click(screen.getByRole('button', { name: en.shorts.build }))
    await waitFor(() => expect(backend.argsOf('run_plugin')).toHaveLength(1))
    expect(backend.argsOf('stage_short_layers')).toEqual([
      {
        workId: IDS.short,
        placed: { ground: 'ground.png' },
        layers: [{ name: 'ground.png', bytes: [1, 2, 3] }],
      },
    ])
    const sent = backend.argsOf('run_plugin')[0]!
    expect(sent).toMatchObject({
      executable: 'kilna-plugin-cut',
      command: 'cut',
      target: 'cuts',
      id: IDS.short,
    })
    await emit('plugin-progress', { run: 'someone-else', progress: 0.9, message: 'elsewhere' })
    expect(screen.queryByText('elsewhere')).toBeNull()

    expect(
      await screen.findByText(en.plugins.madeFinished.replace('{{name}}', 'v1.mp4')),
    ).toBeVisible()
  })

  it('plays the newest build', async () => {
    view = { ...view, file: 'C:/media/shorts/s1/video/v2.mp4' }
    await open()

    expect(
      screen.getByLabelText(en.shorts.builtFile.replace('{{name}}', 'v2.mp4')),
    ).toBeInTheDocument()
  })

  it('keeps a variant given by hand in the short’s fields, and gives it back to the calendar', async () => {
    await open()

    fireEvent.click(screen.getByRole('combobox', { name: en.shorts.variant }))
    choose(await screen.findByRole('option', { name: 'Night' }))
    await waitFor(() => expect(written).toHaveLength(1))
    expect(written[0]?.meta).toEqual({ short_variant: 'night' })

    view = { ...view, variant: { key: 'night', label: 'Night', by_hand: true } }
    fireEvent.click(screen.getByRole('combobox', { name: en.shorts.variant }))
    choose(await screen.findByRole('option', { name: en.shorts.byCalendar }))
    await waitFor(() => expect(written).toHaveLength(2))
    expect(written[1]?.meta).toEqual({ short_variant: null })
  })
})
