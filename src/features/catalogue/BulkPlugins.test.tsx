import { afterEach, describe, expect, it, vi } from 'vitest'
import { MemoryRouter } from 'react-router'
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import type { Plugin, PluginBatchReport } from '@/lib/api/types'
import { Providers } from '@/app/Providers'
import { createQueryClient } from '@/lib/query/client'
import { mockBackend, type Backend } from '@/test/backend'
import { BulkPlugins } from '@/features/catalogue/BulkPlugins'

/*
 * A plugin's batch command over many works (v0.155, FR-21 21.3): chosen,
 * run so many at a time, and reported work by work.
 */

const RENDERER: Plugin = {
  executable: 'kilna-plugin-render',
  path: 'C:/plugins/kilna-plugin-render.exe',
  usable: true,
  reason: null,
  enabled: true,
  settings: {},
  manifest: {
    protocol_version: 1,
    name: 'render',
    version: '0.1.0',
    description: null,
    descriptions: {},
    settings: [],
    commands: [
      {
        key: 'render_headless',
        label: 'Render the videos',
        labels: {},
        description: null,
        descriptions: {},
        target: 'work',
        batch: true,
      },
      {
        key: 'render',
        label: 'Render one',
        labels: {},
        description: null,
        descriptions: {},
        target: 'work',
      },
    ],
  },
}

const REPORT: PluginBatchReport = {
  cancelled: false,
  items: [
    { id: 'w1', title: 'Dawn', ok: true, message: null, files: ['C:/dawn.mp4'] },
    { id: 'w2', title: 'Dusk', ok: false, message: 'no track', files: [] },
  ],
}

let backend: Backend

afterEach(() => {
  vi.restoreAllMocks()
})

describe('a batch of a plugin', () => {
  it('offers only batch commands and runs them so many at a time', async () => {
    backend = mockBackend({
      list_plugins: () => [RENDERER],
      run_plugin_batch: () => REPORT,
    })
    render(
      <Providers client={createQueryClient()}>
        <MemoryRouter>
          <BulkPlugins workIds={['w1', 'w2']} inBar={false} />
        </MemoryRouter>
      </Providers>,
    )

    fireEvent.click(await screen.findByRole('button', { name: 'Plugins…' }))
    expect(screen.queryByRole('menuitem', { name: 'Render one' })).toBeNull()
    fireEvent.click(await screen.findByRole('menuitem', { name: 'Render the videos' }))
    const dialog = await screen.findByRole('dialog', { name: 'Render the videos: 2 works' })
    fireEvent.click(within(dialog).getByRole('radio', { name: '3' }))
    fireEvent.click(within(dialog).getByRole('button', { name: 'Run on 2 works' }))

    await waitFor(() => expect(backend.argsOf('run_plugin_batch')).toHaveLength(1))
    expect(backend.argsOf('run_plugin_batch')[0]).toMatchObject({
      executable: 'kilna-plugin-render',
      command: 'render_headless',
      ids: ['w1', 'w2'],
      parallelism: 3,
    })
    expect(await within(dialog).findByText('Done: 1 worked, 1 failed.')).toBeInTheDocument()
    expect(within(dialog).getByText('no track')).toBeInTheDocument()
  })
})
