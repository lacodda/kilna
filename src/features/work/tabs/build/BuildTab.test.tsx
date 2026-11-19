import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, screen, waitFor } from '@testing-library/react'
import type { WorkPatch } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, simpleVideoPlugin, studio, type Studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The Build tab (v0.149, FR-15): a work that goes out itself has it while
 * the simple-video plugin is installed and on; the timeline set on it is
 * written to the work's `simple_video` after a pause, and "Render the
 * video" runs the plugin's render.
 */

let backend: Backend
let workspace: Studio
let written: WorkPatch[]

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  workspace = studio()
  written = []
  backend = mockBackend({
    ...answersFor(workspace),
    list_plugins: () => [simpleVideoPlugin()],
    build_sources: () => ({
      picture: 'C:/media/cover.png',
      looped: false,
      track: 'C:/media/mix.wav',
    }),
    update_work: ({ id, patch }) => {
      written.push(patch as WorkPatch)
      const work = workspace.works.find((one) => one.id === id)!
      Object.assign(work, { meta: { ...work.meta, ...(patch as WorkPatch).meta } })
      return work
    },
    run_plugin: () => ({
      message: 'Rendered Song.mp4: 3:21',
      files: ['C:/media/Song.mp4'],
      finished: null,
      proposed_score: null,
      candidates: [],
      quota: null,
    }),
  })
})

afterEach(() => {
  vi.useRealTimers()
})

describe('the Build tab', () => {
  it('stands on a work that goes out itself while the renderer is on', async () => {
    const { client } = renderApp(`/works/${IDS.video}/overview`)
    await settled(client)
    expect(
      await screen.findByRole('link', { name: new RegExp(en.card.tab.build) }),
    ).toBeInTheDocument()
  })

  it('writes the timeline after a pause, and renders by the plugin', async () => {
    const { client } = renderApp(`/works/${IDS.video}/build`)
    await settled(client)
    const leadIn = await screen.findByRole('textbox', { name: en.build.leadIn })
    fireEvent.change(leadIn, { target: { value: '2' } })
    fireEvent.blur(leadIn)
    await waitFor(() => expect(written.at(-1)?.meta?.simple_video).toEqual({ lead_in: 2 }), {
      timeout: 3000,
    })

    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: en.build.render }))
    })
    await waitFor(() =>
      expect(backend.argsOf('run_plugin')[0]).toMatchObject({
        executable: 'kilna-plugin-simple-video',
        command: 'render',
        target: 'work',
        id: IDS.video,
      }),
    )
  })
})
