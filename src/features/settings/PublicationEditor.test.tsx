import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, within } from '@testing-library/react'
import type { ProfileConfig, WorkKind } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, NOW, studio, type Studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * What a kind says about the works made from others (v0.86), edited in the
 * profile's draft: the title a made work takes, the shape of each door's
 * cover - refused in the window when it is not one, so the draft never holds
 * a profile that cannot be saved - and whether the kind plays under a frame.
 */

let backend: Backend
let workspace: Studio

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  workspace = studio()
  // The audio kind alone, and two actions: every box of the profile editor is
  // measured by jsdom on every render, and what is tested here is one kind.
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

async function openProfile() {
  const view = renderApp('/settings/profile')
  await settled(view.client)
  const madeTitle = await screen.findByRole('textbox', { name: en.editor.madeTitle })
  return { ...view, madeTitle }
}

/** Save the draft from the bar, and answer with the audio kind as it went. */
async function saveAudio(client: Awaited<ReturnType<typeof openProfile>>['client']) {
  fireEvent.click(within(bar()!).getByRole('button', { name: en.editor.save }))
  await settled(client)
  const [saved] = backend.argsOf('update_profile_config')
  return (saved!.config as ProfileConfig).work_kinds.find((kind) => kind.key === 'audio')!
}

const door = (kind: WorkKind, key: string) => kind.release_kinds!.find((d) => d.key === key)!

describe('the title of a made work', () => {
  it('is written into the draft as one word, and refused without {title}', async () => {
    const { client, madeTitle } = await openProfile()
    expect(madeTitle).toHaveValue('{title} — audio')

    fireEvent.change(madeTitle, { target: { value: 'Lanterns' } })
    expect(screen.getByText(en.editor.madeTitleNoTitle)).toBeInTheDocument()
    expect(bar(), 'a refused title reached the draft').toBeNull()

    fireEvent.change(madeTitle, { target: { value: '{title} (audio {n})' } })
    expect(screen.queryByText(en.editor.madeTitleNoTitle)).toBeNull()
    expect(bar()).not.toBeNull()

    const audio = await saveAudio(client)
    expect(audio.made_title).toBe('{title} (audio {n})')
  })

  it('is removed when cleared', async () => {
    const { client, madeTitle } = await openProfile()

    fireEvent.change(madeTitle, { target: { value: '' } })

    const audio = await saveAudio(client)
    expect(audio.made_title ?? null).toBeNull()
  })
})

describe("a door's cover shape", () => {
  it('is kept only as two whole numbers with a colon', async () => {
    const { client } = await openProfile()
    const youtube = screen.getByRole('textbox', { name: 'YouTube' })
    expect(youtube).toHaveValue('16:9')

    for (const refused of ['16:', '16x9', '0:9', '4:5:1']) {
      fireEvent.change(youtube, { target: { value: refused } })
      expect(youtube).toHaveValue(refused)
      expect(screen.getByText(en.editor.coverFormatInvalid)).toBeInTheDocument()
      expect(bar(), `${refused} reached the draft`).toBeNull()
    }

    fireEvent.change(youtube, { target: { value: '4:5' } })
    expect(screen.queryByText(en.editor.coverFormatInvalid)).toBeNull()
    // Emptied, a door shows no picture of its own.
    fireEvent.change(screen.getByRole('textbox', { name: 'Streaming' }), { target: { value: '' } })

    const audio = await saveAudio(client)
    expect(door(audio, 'youtube').cover_format).toBe('4:5')
    expect(door(audio, 'streaming').cover_format ?? null).toBeNull()
    // The keys are the document's, and stay as they were.
    expect(audio.release_kinds!.map((d) => d.key)).toEqual(['youtube', 'streaming'])
  })
})

describe('a kind with a frame', () => {
  it('says so, and offers nothing to change it', async () => {
    await openProfile()
    expect(screen.getByText(en.editor.frameLine)).toBeInTheDocument()
  })
})
