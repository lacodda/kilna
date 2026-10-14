import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, within } from '@testing-library/react'
import type { ProfileConfig, WorkKind } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, NOW, studio, type Studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * What each door will not let a release out without (v0.97), ticked in the
 * profile's draft: a version of a role, a cover, a field written. Unticking
 * writes `false` rather than dropping the word, so the upgrade that carries
 * the shipped profile forward at the next start does not tick it back.
 */

let backend: Backend
let workspace: Studio

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  workspace = studio()
  // The audio kind alone: what is tested here is one kind's doors.
  const config = workspace.profile.config
  config.work_kinds = config.work_kinds.filter((kind) => kind.key === 'audio')
  config.prompts = config.prompts.slice(0, 2)
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

const bar = () =>
  document.querySelector<HTMLElement>(`[role="toolbar"][aria-label="${en.settings.unsaved}"]`)

const door = (kind: WorkKind, key: string) => kind.release_kinds!.find((d) => d.key === key)!

/** The block of one door in the checklist editor, found by its name. */
async function doorBlock(label: string) {
  return screen.findByRole('group', {
    name: en.editor.readinessOf.replace('{{door}}', label),
  })
}

describe('the checklist of a door', () => {
  it('shows what the shipped door asks for, and writes a box taken off as false', async () => {
    const { client } = renderApp('/settings/profile')
    await settled(client)

    const streaming = await doorBlock('Streaming')
    const cover = within(streaming).getByRole('checkbox', { name: en.readiness.cover })
    const title = within(streaming).getByRole('checkbox', { name: 'Title' })
    expect(cover).toBeChecked()
    expect(title).toBeChecked()

    fireEvent.click(cover)
    expect(bar()).not.toBeNull()
    fireEvent.click(within(bar()!).getByRole('button', { name: en.editor.save }))
    await settled(client)

    const [saved] = backend.argsOf('update_profile_config')
    const audio = (saved!.config as ProfileConfig).work_kinds.find((kind) => kind.key === 'audio')!
    expect(door(audio, 'streaming').requires_cover).toBe(false)
    expect(door(audio, 'streaming').fields!.find((field) => field.key === 'title')?.required).toBe(
      true,
    )
    expect(door(audio, 'youtube').requires_cover, 'the other door is left alone').toBe(true)
  })

  it('makes a field required, and a role a requirement of the door', async () => {
    const { client } = renderApp('/settings/profile')
    await settled(client)

    const youtube = await doorBlock('YouTube')
    fireEvent.click(within(youtube).getByRole('checkbox', { name: 'Tags' }))
    fireEvent.click(within(youtube).getByRole('checkbox', { name: 'Concept' }))
    fireEvent.click(within(bar()!).getByRole('button', { name: en.editor.save }))
    await settled(client)

    const [saved] = backend.argsOf('update_profile_config')
    const audio = (saved!.config as ProfileConfig).work_kinds.find((kind) => kind.key === 'audio')!
    expect(door(audio, 'youtube').fields!.find((field) => field.key === 'tags')?.required).toBe(
      true,
    )
    expect(door(audio, 'youtube').requires).toEqual(['plot'])
  })
})
