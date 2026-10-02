import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, within } from '@testing-library/react'
import type { ProfileConfig } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio, type Studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The overview's layout, chosen in Settings -> The work card (v0.82).
 *
 * The profile holds it, so choosing one writes the profile at once - the
 * layout alone, the placements left as they were - and a profile that never
 * chose shows the owner's choice, the lead column and its rail.
 */

let backend: Backend
let workspace: Studio

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  workspace = studio()
  backend = mockBackend({
    ...answersFor(workspace),
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

async function openCard() {
  const view = renderApp('/settings/card')
  await settled(view.client)
  return screen.findByRole('radiogroup', { name: en.settings.overviewLayoutLabel })
}

const names = en.settings.overviewLayout

describe('the overview layout', () => {
  it('offers the five layouts and starts on the lead one', async () => {
    const group = await openCard()

    const options = within(group).getAllByRole('radio')
    expect(options.map((option) => option.textContent)).toEqual([
      names.grid,
      names.lead,
      names.bands,
      names.mosaic,
      names.sheet,
    ])
    expect(within(group).getByRole('radio', { name: names.lead })).toHaveAttribute(
      'aria-checked',
      'true',
    )
  })

  it('writes the chosen layout into the profile and keeps the placements', async () => {
    // A config of its own: the fixture's is the imported profile, shared by
    // every test in the file.
    workspace.profile = {
      ...workspace.profile,
      config: {
        ...workspace.profile.config,
        overview: { layout: 'lead', widgets: [{ id: 'score', size: 'l', position: 0 }] },
      },
    }
    const group = await openCard()

    fireEvent.click(within(group).getByRole('radio', { name: names.mosaic }))

    await vi.waitFor(() => expect(backend.argsOf('update_profile_config')).toHaveLength(1))
    const [sent] = backend.argsOf('update_profile_config')
    const config = sent!.config as ProfileConfig
    expect(config.overview).toEqual({
      layout: 'mosaic',
      widgets: [{ id: 'score', size: 'l', position: 0 }],
    })
    // Nothing else in the document moves: the rest is what was stored.
    expect(config.work_kinds).toEqual(workspace.profile.config.work_kinds)
    await vi.waitFor(() =>
      expect(within(group).getByRole('radio', { name: names.mosaic })).toHaveAttribute(
        'aria-checked',
        'true',
      ),
    )
  })

  it('gives a profile that never chose an empty placement, the one kilna ships', async () => {
    const group = await openCard()

    fireEvent.click(within(group).getByRole('radio', { name: names.sheet }))

    await vi.waitFor(() => expect(backend.argsOf('update_profile_config')).toHaveLength(1))
    const config = backend.argsOf('update_profile_config')[0]!.config as ProfileConfig
    expect(config.overview).toEqual({ layout: 'sheet', widgets: [] })
  })
})

/** Press an option the way a pointer does: Base UI picks on the press. */
function choose(option: HTMLElement) {
  fireEvent.pointerDown(option, { pointerType: 'mouse' })
  fireEvent.click(option)
}

const openOn = (kind: string) => en.settings.defaultTabFor.replace('{{kind}}', kind)

describe('where a card opens', () => {
  it('offers each kind the tabs its works draw, and writes the choice to that kind alone', async () => {
    await openCard()

    // A song is offered its versions, and no board it has not got.
    fireEvent.click(screen.getByRole('combobox', { name: openOn('Song') }))
    const offered = (await screen.findAllByRole('option')).map((option) => option.textContent)
    expect(offered).toContain(en.card.tab.versions)
    expect(offered).not.toContain(en.card.tab.scenes)
    choose(screen.getByRole('option', { name: en.card.tab.versions }))

    await vi.waitFor(() => expect(backend.argsOf('update_profile_config')).toHaveLength(1))
    const config = backend.argsOf('update_profile_config')[0]!.config as ProfileConfig
    const kinds = Object.fromEntries(config.work_kinds.map((kind) => [kind.key, kind.open_on]))
    expect(kinds.song).toBe('versions')
    // Every other kind keeps opening where it did.
    for (const [key, tab] of Object.entries(kinds)) if (key !== 'song') expect(tab).toBeUndefined()
    await vi.waitFor(() =>
      expect(screen.getByRole('combobox', { name: openOn('Song') })).toHaveTextContent(
        en.card.tab.versions,
      ),
    )
  })

  it('writes the overview as nothing, the way a kind that never chose says it', async () => {
    workspace.profile = {
      ...workspace.profile,
      config: {
        ...workspace.profile.config,
        work_kinds: workspace.profile.config.work_kinds.map((kind) =>
          kind.key === 'short' ? { ...kind, open_on: 'scenes' } : kind,
        ),
      },
    }
    await openCard()

    const short = screen.getByRole('combobox', { name: openOn('Short') })
    expect(short).toHaveTextContent(en.card.tab.scenes)
    fireEvent.click(short)
    choose(await screen.findByRole('option', { name: en.card.tab.overview }))

    await vi.waitFor(() => expect(backend.argsOf('update_profile_config')).toHaveLength(1))
    const config = backend.argsOf('update_profile_config')[0]!.config as ProfileConfig
    expect(config.work_kinds.find((kind) => kind.key === 'short')!.open_on).toBeNull()
  })
})

describe('a card opened with no tab in its address', () => {
  it('opens on the tab its kind names', async () => {
    workspace.profile = {
      ...workspace.profile,
      config: {
        ...workspace.profile.config,
        work_kinds: workspace.profile.config.work_kinds.map((kind) =>
          kind.key === 'song' ? { ...kind, open_on: 'versions' } : kind,
        ),
      },
    }
    const view = renderApp(`/works/${IDS.song}`)
    await settled(view.client)
    const main = await screen.findByRole('main')
    await vi.waitFor(() =>
      expect(within(main).getByRole('link', { current: 'page' })).toHaveAttribute(
        'href',
        `/works/${IDS.song}/versions`,
      ),
    )

    // A clip's kind chose nothing: its card opens on the overview.
    view.unmount()
    const clip = renderApp(`/works/${IDS.video}`)
    await settled(clip.client)
    const opened = await screen.findByRole('main')
    await vi.waitFor(() =>
      expect(within(opened).getByRole('link', { current: 'page' })).toHaveAttribute(
        'href',
        `/works/${IDS.video}/overview`,
      ),
    )
  })
})
