import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { MemoryRouter } from 'react-router'
import type { Plugin } from '@/lib/api/types'
import { Providers } from '@/app/Providers'
import { PluginBar } from '@/features/work/PluginBar'
import i18n from '@/i18n'
import type { PluginTarget } from '@/lib/api/plugins'
import { createQueryClient } from '@/lib/query/client'
import { mockBackend, type Backend } from '@/test/backend'
import en from '@/i18n/locales/en.json'

/*
 * Where a plugin's commands stand (v0.108): one on a release at last, a judge on
 * the Score tab, each in the window's language, and none from a plugin the
 * person turned off. A plugin that cannot run no longer says why in the head of
 * every work: it says that it cannot, and where to read why.
 */

const JUDGE: Plugin = {
  executable: 'kilna-plugin-judge',
  path: 'C:/plugins/kilna-plugin-judge.exe',
  usable: true,
  reason: null,
  enabled: true,
  settings: {},
  manifest: {
    protocol_version: 1,
    name: 'judge',
    version: '0.1.0',
    description: null,
    descriptions: {},
    settings: [],
    commands: [
      {
        key: 'judge',
        label: 'Judge it',
        labels: { ru: 'Оценить' },
        description: 'Judges along the axes.',
        descriptions: { ru: 'Оценивает по осям.' },
        target: 'score',
      },
      {
        key: 'stats',
        label: 'Fetch the numbers',
        labels: {},
        description: null,
        descriptions: {},
        target: 'release',
      },
    ],
  },
}

let backend: Backend

function mount(target: PluginTarget, plugins: Plugin[] = [JUDGE]) {
  backend = mockBackend({
    list_plugins: () => plugins,
    run_plugin: () => ({ message: null, files: [], finished: null, proposed_score: null }),
  })
  return render(
    <Providers client={createQueryClient()}>
      <MemoryRouter>
        <PluginBar target={target} id="w1" />
      </MemoryRouter>
    </Providers>,
  )
}

beforeEach(async () => {
  await i18n.changeLanguage('en')
})

afterEach(async () => {
  vi.restoreAllMocks()
  await i18n.changeLanguage('en')
})

describe('the plugin bar', () => {
  it('offers on a release what is meant for a release, and only that', async () => {
    mount('release')

    const button = await screen.findByRole('button', { name: 'Fetch the numbers' })
    expect(screen.queryByRole('button', { name: 'Judge it' })).toBeNull()

    fireEvent.click(button)
    await waitFor(() => expect(backend.argsOf('run_plugin')).toHaveLength(1))

    expect(backend.argsOf('run_plugin')[0]).toMatchObject({
      executable: JUDGE.executable,
      command: 'stats',
      target: 'release',
      id: 'w1',
    })
  })

  it('offers a judge on the Score tab, in the window’s language', async () => {
    await i18n.changeLanguage('ru')
    mount('score')

    const button = await screen.findByRole('button', { name: 'Оценить' })

    expect(button).toHaveAttribute('title', 'Оценивает по осям.')
  })

  it('offers nothing from a plugin that was turned off', async () => {
    mount('score', [{ ...JUDGE, enabled: false }])

    await waitFor(() => expect(backend.argsOf('list_plugins')).toHaveLength(1))
    await waitFor(() => expect(screen.queryByRole('group')).toBeNull())
    expect(screen.queryByRole('button', { name: 'Judge it' })).toBeNull()
  })

  it('says that a plugin cannot run and where to read why, not the why', async () => {
    const reason = 'its manifest could not be read'
    mount('work', [
      { ...JUDGE, executable: 'kilna-plugin-a', usable: false, manifest: null, reason },
      { ...JUDGE, executable: 'kilna-plugin-b', usable: false, manifest: null, reason },
    ])

    const link = await screen.findByRole('link', { name: '2 plugins cannot run' })

    expect(link).toHaveAttribute('href', '/settings/plugins')
    expect(screen.queryByText(new RegExp(reason))).toBeNull()
    expect(en.plugins.cannotRun_other).toContain('{{count}}')
  })

  it('says so once on a card: in the head of the work, not on each of its bars', async () => {
    mount('release', [{ ...JUDGE, usable: false, manifest: null, reason: 'x' }])

    await waitFor(() => expect(backend.argsOf('list_plugins')).toHaveLength(1))
    await waitFor(() => expect(screen.queryByRole('group')).toBeNull())
    expect(screen.queryByRole('link')).toBeNull()
  })

  it('leaves alone a plugin that cannot run and was turned off', async () => {
    mount('work', [{ ...JUDGE, usable: false, manifest: null, reason: 'x', enabled: false }])

    await waitFor(() => expect(backend.argsOf('list_plugins')).toHaveLength(1))
    await waitFor(() => expect(screen.queryByRole('group')).toBeNull())
    expect(screen.queryByRole('link')).toBeNull()
  })
})
