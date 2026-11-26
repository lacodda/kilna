import { beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, waitFor, within } from '@testing-library/react'
import type { ProfileConfig } from '@/lib/api/types'
import { keepProfileDraft } from '@/lib/profileDraft'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, studio, type Studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

// A frame is drawn on a canvas, which this window has not got: what is drawn
// is the layers' own test's to hold.
vi.mock('@/lib/shorts/layers', () => ({
  draw: () => Promise.resolve({}),
  paint: () => undefined,
  pictureOf: () => Promise.reject(new Error('no pictures here')),
}))

/*
 * Settings -> Shorts (v0.168): the constructor of the space's short style.
 * What is edited is the profile's draft, saved from the bar like the rest of
 * the profile; a space that set no style opens on kilna's own.
 */

let backend: Backend
let workspace: Studio
let saved: ProfileConfig[]

beforeEach(() => {
  workspace = studio()
  saved = []
  keepProfileDraft(workspace.profile.id, null)
  backend = mockBackend({
    ...answersFor(workspace),
    update_profile_config: ({ config }) => {
      saved.push(config as ProfileConfig)
      workspace.profile.config = config as ProfileConfig
      return workspace.profile
    },
  })
})

const open = async () => {
  const app = renderApp('/settings/shorts')
  await screen.findByRole('region', { name: en.shorts.header })
  await settled(app.client)
  return app
}

const bar = () =>
  document.querySelector<HTMLElement>(`[role="toolbar"][aria-label="${en.settings.unsaved}"]`)

describe('the short style constructor', () => {
  it('opens on kilna’s own style, and saves the first change as the space’s', async () => {
    await open()
    expect(backend.argsOf('short_style_default')).toHaveLength(1)
    expect(bar(), 'a bar before anything was edited').toBeNull()

    const header = screen.getByRole('region', { name: en.shorts.header })
    const call = within(header).getByRole('textbox', { name: en.shorts.call })
    expect(call).toHaveValue('the whole song is on the channel')
    fireEvent.change(call, { target: { value: 'hear it whole' } })

    await waitFor(() => expect(bar()).not.toBeNull())
    fireEvent.click(within(bar()!).getByRole('button', { name: en.editor.save }))
    await waitFor(() => expect(saved).toHaveLength(1))
    const style = saved[0]?.shorts
    expect(style?.header.call).toBe('hear it whole')
    // The rest of the style went with it, whole: the space has a style now.
    expect(style?.wave.kind).toBe('solid')
    expect(style?.ground.blots).toHaveLength(1)
  })

  it('turns a block off by its box', async () => {
    await open()
    const wave = screen.getByRole('region', { name: en.shorts.wave })
    fireEvent.click(within(wave).getByRole('checkbox', { name: en.shorts.on }))

    await waitFor(() => expect(bar()).not.toBeNull())
    fireEvent.click(within(bar()!).getByRole('button', { name: en.editor.save }))
    await waitFor(() => expect(saved).toHaveLength(1))
    expect(saved[0]?.shorts?.wave.on).toBe(false)
  })

  it('adds a variant that replaces only what its boxes say', async () => {
    await open()
    expect(screen.getByText(en.shorts.noVariants)).toBeVisible()
    expect(screen.getByText(en.shorts.shownStyle)).toBeVisible()

    fireEvent.click(screen.getByRole('button', { name: en.shorts.addVariant }))
    const name = await screen.findByRole('textbox', { name: en.shorts.variantLabel })
    fireEvent.change(name, { target: { value: 'Night' } })
    expect(
      screen.getByText(en.shorts.shownVariant.replace('{{name}}', 'Night')),
      'the frame is drawn as the variant wears it',
    ).toBeVisible()
    // Nothing is its own yet: every value is the style's.
    expect(screen.getAllByText(en.shorts.asStyle).length).toBeGreaterThan(4)

    const variants = screen.getByRole('region', { name: en.shorts.variants })
    fireEvent.click(within(variants).getByRole('checkbox', { name: en.shorts.stampAngle }))

    fireEvent.click(within(bar()!).getByRole('button', { name: en.editor.save }))
    await waitFor(() => expect(saved).toHaveLength(1))
    expect(saved[0]?.shorts?.variants).toEqual([
      { key: 'variant-1', label: 'Night', stamp_angle: -6 },
    ])
  })

  it('is not listed in a space whose craft cuts no shorts', async () => {
    for (const kind of workspace.profile.config.work_kinds) kind.excerpt = false
    const app = renderApp('/settings/general')
    await screen.findByText(en.settings.appearance)
    await settled(app.client)

    expect(document.querySelector('main nav a[href="/settings/shorts"]')).toBeNull()
    expect(document.querySelector('main nav a[href="/settings/plugins"]')).not.toBeNull()
  })
})
