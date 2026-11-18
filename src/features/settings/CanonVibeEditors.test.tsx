import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, within } from '@testing-library/react'
import type { ProfileConfig } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { openProfilePart } from '@/test/profile'
import { renderApp, settled } from '@/test/render'
import { answersFor, NOW, studio, type Studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The structure of the canon and the lines of the vibe, edited in the
 * profile (v0.129, FR-16): a card's sections renamed and reshaped, a word of
 * the vibe given its stroke count, a mark its glyph.
 */

let backend: Backend
let workspace: Studio

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  workspace = studio()
  backend = mockBackend({
    ...answersFor(workspace),
    check_profile_config: () => [],
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

async function open(part: string) {
  const view = renderApp('/settings/profile')
  await settled(view.client)
  await openProfilePart(part)
  return view
}

async function saved(client: Awaited<ReturnType<typeof open>>['client']) {
  const bar = await screen.findByRole('toolbar', { name: en.settings.unsaved })
  fireEvent.click(within(bar).getByRole('button', { name: en.editor.save }))
  await settled(client)
  const [last] = backend.argsOf('update_profile_config').slice(-1)
  return last!.config as ProfileConfig
}

describe('the canon and the vibe in the profile', () => {
  it('renames a section of a card of the canon, each with its shape', async () => {
    const card = workspace.profile.config.note_kinds!.find(
      (kind) => (kind.sections ?? []).length > 0,
    )!
    const section = card.sections![0]!
    const kindWord = typeof card.label === 'string' ? card.label : card.label.en!
    const { client } = await open(en.editor.part.canon)
    const named = (text: string) =>
      text.replace('{{key}}', section.key).replace('{{kind}}', kindWord)
    expect(screen.getByRole('combobox', { name: named(en.editor.canon.shapeOf) })).toBeVisible()
    fireEvent.change(screen.getByRole('textbox', { name: named(en.editor.canon.wordOf) }), {
      target: { value: 'Who they are' },
    })
    const config = await saved(client)
    const after = config.note_kinds!.find((kind) => kind.key === card.key)!
    const label = after.sections![0]!.label
    expect(typeof label === 'string' ? label : label.en).toBe('Who they are')
  })

  it('renames a word of the vibe, each with its shape and its strokes', async () => {
    const vibe = workspace.profile.config.vibe!
    // Joined so the colour check of the source does not read it as a style.
    const colour = ['#', '123456'].join('')
    const character = vibe.characters[0]!
    const complexity = vibe.complexities[0]!
    const { client } = await open(en.editor.part.vibe)
    expect(
      screen.getByRole('combobox', {
        name: en.editor.vibe.glyphOf.replace('{{key}}', character.key),
      }),
    ).toBeVisible()
    expect(
      screen.getByRole('combobox', {
        name: en.editor.vibe.strokesOf.replace('{{key}}', complexity.key),
      }),
    ).toBeVisible()
    fireEvent.change(
      screen.getByRole('textbox', {
        name: en.editor.vibe.colourOf.replace('{{key}}', vibe.moods[0]!.key),
      }),
      { target: { value: colour } },
    )
    const config = await saved(client)
    expect(config.vibe!.moods[0]!.color).toBe(colour)
  })

  it('offers a glyph for every mark', async () => {
    const mark = workspace.profile.config.marks![0]!
    await open(en.editor.part.craft)
    expect(
      screen.getByRole('combobox', { name: en.editor.markIconOf.replace('{{key}}', mark.key) }),
    ).toBeVisible()
  })
})
