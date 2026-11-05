import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, within } from '@testing-library/react'
import type { Plugin } from '@/lib/api/types'
import { inLanguage } from '@/lib/pluginWords'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The plugins' section of Settings (v0.108): what is installed, whether it can
 * run, where its commands stand, and its settings, drawn from what its manifest
 * declares. What is on the screen is what a run is told: what was set here, or
 * the plugin's own default.
 */

const CUTTER: Plugin = {
  executable: 'kilna-plugin-cut',
  path: 'C:/workspace/plugins/kilna-plugin-cut.exe',
  usable: true,
  reason: null,
  enabled: true,
  settings: { crf: 18 },
  manifest: {
    protocol_version: 1,
    name: 'cut',
    version: '0.3.0',
    description: 'Cuts a short.',
    descriptions: {},
    commands: [
      {
        key: 'cut',
        label: 'Cut the short',
        labels: {},
        description: null,
        descriptions: {},
        target: 'cuts',
      },
      {
        key: 'judge',
        label: 'Judge it',
        labels: {},
        description: null,
        descriptions: {},
        target: 'score',
      },
    ],
    settings: [
      {
        key: 'channel',
        label: 'Channel',
        labels: {},
        descriptions: {},
        type: 'text',
        default: 'Tide',
      },
      {
        key: 'crf',
        label: 'Quality',
        labels: {},
        descriptions: {},
        type: 'number',
        min: 0,
        max: 51,
        default: 20,
      },
      {
        key: 'hand',
        label: 'The pointing hand',
        labels: {},
        descriptions: {},
        type: 'boolean',
        default: true,
      },
    ],
  },
}

const BROKEN: Plugin = {
  executable: 'kilna-plugin-broken',
  path: 'C:/workspace/plugins/kilna-plugin-broken.cmd',
  usable: false,
  reason: 'its manifest could not be read: expected value',
  enabled: true,
  settings: {},
  manifest: null,
}

let backend: Backend

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  backend = mockBackend({
    ...answersFor(studio()),
    list_plugins: () => [BROKEN, CUTTER],
    set_plugin_enabled: () => null,
    set_plugin_setting: () => null,
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

async function open() {
  const { client } = renderApp('/settings/plugins')
  await settled(client)
  const main = await screen.findByRole('main')
  await within(main).findByText(en.plugins.folder)
  await settled(client)
  return { client, main }
}

/** One plugin's block on the screen, found by its executable. */
function block(main: HTMLElement, executable: string): HTMLElement {
  const found = main.querySelector<HTMLElement>(`[data-plugin="${executable}"]`)
  expect(found, `no block for ${executable}`).not.toBeNull()
  return found!
}

describe('the plugins in Settings', () => {
  it('says what each is, where its commands stand, and why one cannot run', async () => {
    const { main } = await open()

    const cutter = within(block(main, CUTTER.executable))
    expect(cutter.getByText('0.3.0')).toBeVisible()
    expect(cutter.getByText('Cuts a short.')).toBeVisible()
    expect(cutter.getByText(en.plugins.state.on)).toBeVisible()
    expect(cutter.getByText('Cut the short')).toBeVisible()
    expect(cutter.getByText(en.plugins.where.cuts)).toBeVisible()
    expect(cutter.getByText(en.plugins.where.score)).toBeVisible()

    const broken = within(block(main, BROKEN.executable))
    expect(broken.getByText(en.plugins.state.unusable)).toBeVisible()
    expect(broken.getByText(BROKEN.reason!)).toBeVisible()
  })

  it('says where a plugin goes when there are none, and how one is written', async () => {
    backend.answer('list_plugins', () => [])
    const { main } = await open()

    expect(within(main).getByText(en.plugins.noneTitle)).toBeVisible()
    expect(within(main).getByText('C:/workspace/plugins')).toBeVisible()
    expect(within(main).getByRole('button', { name: en.plugins.guide })).toBeVisible()
  })

  it('looks again when told to: a plugin dropped in the folder a moment ago', async () => {
    backend.answer('list_plugins', () => [])
    const { client, main } = await open()

    backend.answer('list_plugins', () => [CUTTER])
    fireEvent.click(within(main).getByRole('button', { name: en.plugins.lookAgain }))
    await settled(client)

    expect(within(main).queryByText(en.plugins.noneTitle)).toBeNull()
    expect(block(main, CUTTER.executable)).toBeVisible()
  })

  it('turns a plugin off, and leaves it listed', async () => {
    const { client, main } = await open()

    fireEvent.click(
      within(block(main, CUTTER.executable)).getByRole('switch', { name: en.plugins.enabled }),
    )
    await settled(client)

    expect(backend.argsOf('set_plugin_enabled')).toEqual([
      { executable: CUTTER.executable, enabled: false },
    ])

    backend.answer('list_plugins', () => [{ ...CUTTER, enabled: false }])
    fireEvent.click(within(main).getByRole('button', { name: en.plugins.lookAgain }))
    await settled(client)
    expect(within(block(main, CUTTER.executable)).getByText(en.plugins.state.off)).toBeVisible()
  })

  it('shows what a run is told: what was set here, or the plugin’s default', async () => {
    const { main } = await open()
    const cutter = within(block(main, CUTTER.executable))

    expect(cutter.getByRole('textbox', { name: 'Channel' })).toHaveValue('Tide')
    expect(cutter.getByRole('textbox', { name: 'Quality' })).toHaveValue('18')
    expect(cutter.getByRole('switch', { name: 'The pointing hand' })).toBeChecked()
  })

  it('saves a setting on leaving it, and writes nothing unchanged', async () => {
    const { client, main } = await open()
    const box = () =>
      within(block(main, CUTTER.executable)).getByRole('textbox', { name: 'Channel' })

    fireEvent.blur(box())
    await settled(client)
    expect(backend.argsOf('set_plugin_setting')).toEqual([])

    fireEvent.change(box(), { target: { value: 'Sea' } })
    fireEvent.blur(box())
    await settled(client)

    expect(backend.argsOf('set_plugin_setting')).toEqual([
      { executable: CUTTER.executable, key: 'channel', value: 'Sea' },
    ])
  })

  it('takes back only a value that was set', async () => {
    const { client, main } = await open()
    const cutter = within(block(main, CUTTER.executable))

    // `crf` was set here; `channel` and `hand` follow the plugin.
    const back = cutter.getAllByRole('button', { name: en.plugins.backToDefault })
    expect(back).toHaveLength(1)

    fireEvent.click(back[0]!)
    await settled(client)

    expect(backend.argsOf('set_plugin_setting')).toEqual([
      { executable: CUTTER.executable, key: 'crf', value: null },
    ])
  })

  it('sends a yes or no at once', async () => {
    const { client, main } = await open()

    fireEvent.click(
      within(block(main, CUTTER.executable)).getByRole('switch', { name: 'The pointing hand' }),
    )
    await settled(client)

    expect(backend.argsOf('set_plugin_setting')).toEqual([
      { executable: CUTTER.executable, key: 'hand', value: false },
    ])
  })
})

describe('a plugin’s word in the window’s language', () => {
  it('takes the language, then its base, then the plugin’s own word', () => {
    expect(inLanguage('ru', 'Cut', { ru: 'Нарезать' })).toBe('Нарезать')
    expect(inLanguage('ru-RU', 'Cut', { ru: 'Нарезать' })).toBe('Нарезать')
    expect(inLanguage('ru-RU', 'Cut', { 'ru-RU': 'Резать', ru: 'Нарезать' })).toBe('Резать')
    expect(inLanguage('en', 'Cut', { ru: 'Нарезать' })).toBe('Cut')
    expect(inLanguage('ru', 'Cut', {})).toBe('Cut')
  })
})
