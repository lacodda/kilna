import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, waitFor, within } from '@testing-library/react'
import type { WorkPatch } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio, type Studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The Cover tab (v0.86): the shapes a publication's doors ask for, lit where
 * it already goes out, and the prompt its cover is drawn from - which the
 * Files tab no longer carries.
 */

let backend: Backend
let workspace: Studio

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  workspace = studio()
  backend = mockBackend({
    ...answersFor(workspace),
    update_work: ({ id, patch }) => {
      const work = workspace.works.find((one) => one.id === id)!
      Object.assign(work, patch as WorkPatch)
      return work
    },
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

async function openCover(workId: string) {
  const { client } = renderApp(`/works/${workId}/cover`)
  await screen.findByRole('region', { name: en.cover.formats })
  await settled(client)
}

/** The entry of the formats strip that names `format`. */
function entry(format: string): HTMLElement {
  const strip = screen.getByRole('region', { name: en.cover.formats })
  return within(strip).getByText(format).closest('li')!
}

describe('the formats', () => {
  it('light the door the work goes out through, and say of the other that nothing does yet', async () => {
    // The audio holds one release, on YouTube; its kind names streaming too.
    await openCover(IDS.audio)

    expect(within(entry('YouTube · 16:9')).queryByText(en.cover.noRelease)).toBeNull()
    expect(within(entry('Streaming · 1:1')).getByText(en.cover.noRelease)).toBeInTheDocument()
    expect(screen.getByText(en.cover.formatsHint)).toBeInTheDocument()
  })

  it('light a door as soon as a release goes out through it', async () => {
    workspace.releases.push({
      ...workspace.releases.find((release) => release.id === IDS.audioRelease)!,
      id: 'r-lanterns-streaming',
      kind: 'streaming',
    })
    await openCover(IDS.audio)

    expect(within(entry('Streaming · 1:1')).queryByText(en.cover.noRelease)).toBeNull()
  })
})

describe('the cover prompt', () => {
  it('stands on the tab, one box per part, and saves the whole set', async () => {
    await openCover(IDS.audio)

    const picture = screen.getByRole('textbox', { name: 'Picture' })
    expect(picture).toHaveValue('a paper lantern on dark water, seen from above')
    expect(screen.getByRole('textbox', { name: 'Negative' })).toBeInTheDocument()
    const typography = screen.getByRole('textbox', { name: 'Typography' })

    fireEvent.change(typography, { target: { value: 'the title, small, bottom left' } })
    fireEvent.blur(typography)

    await waitFor(() =>
      expect(backend.argsOf('update_work').map((args) => (args.patch as WorkPatch).cover)).toEqual([
        {
          picture: 'a paper lantern on dark water, seen from above',
          typography: 'the title, small, bottom left',
        },
      ]),
    )
  })

  it('is no longer on the Files tab, which keeps its gallery', async () => {
    const { client } = renderApp(`/works/${IDS.audio}/files`)
    await screen.findByText(en.files.empty)
    await settled(client)

    expect(screen.queryByText(en.cover.hint)).toBeNull()
    expect(screen.queryByRole('textbox', { name: 'Picture' })).toBeNull()
    expect(screen.getByRole('button', { name: en.files.setCover })).toBeInTheDocument()
  })
})
