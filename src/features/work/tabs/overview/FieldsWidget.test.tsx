import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, screen, waitFor, within } from '@testing-library/react'
import type { WorkPatch } from '@/lib/api/types'
import { mockBackend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio, type Studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The fields widget reads the fields of the work's own kind (v0.86): the
 * audio release's variant - a choice of the shipped Studio profile, for audio
 * alone - is picked from its answers there, stored by key and read by label,
 * and is no box at all on the song it was made from.
 */

let workspace: Studio
let written: WorkPatch[]

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  workspace = studio()
  written = []
})

afterEach(() => {
  vi.useRealTimers()
})

/** Choose an answer the way a pointer does: the press starts on the item,
 *  which is what the picker asks of a mouse before it takes the click. */
function choose(option: HTMLElement) {
  fireEvent.pointerDown(option, { pointerType: 'mouse' })
  fireEvent.click(option)
}

/** The fields widget on the overview of `workId`, once everything answered. */
async function openFields(workId: string, kind: string) {
  mockBackend({
    ...answersFor(workspace),
    // Given the way the backend gives it: the next code, to the works named
    // that have none.
    number_works: ({ field, workIds }) => {
      for (const id of workIds as string[]) {
        const index = workspace.works.findIndex((w) => w.id === id)
        const work = workspace.works[index]!
        workspace.works[index] = { ...work, meta: { ...work.meta, [field as string]: 'CAT-001' } }
      }
      return { changed: (workIds as string[]).length, skipped: [] }
    },
    // Kept the way the backend keeps it - fields merged by key, `null`
    // emptying one - so the reads after the write see it.
    update_work: ({ id, patch }) => {
      const change = patch as WorkPatch
      written.push(change)
      const index = workspace.works.findIndex((w) => w.id === id)
      const work = workspace.works[index]!
      const meta = { ...work.meta }
      for (const [key, value] of Object.entries(change.meta ?? {})) {
        if (value === null) delete meta[key]
        else meta[key] = value
      }
      workspace.works[index] = { ...work, meta }
      return workspace.works[index]
    },
  })
  const view = renderApp(`/works/${workId}/overview`)
  await settled(view.client)
  const main = await screen.findByRole('main')
  const widget = await within(main).findByRole('group', {
    name: en.overview.fields.replace('{{kind}}', kind),
  })
  await settled(view.client)
  return widget
}

describe('the fields of a kind', () => {
  it('picks a choice from its answers, read by label and stored by key', async () => {
    const widget = await openFields(IDS.audio, 'Audio')

    const variant = within(widget).getByRole('combobox', { name: 'Variant' })
    expect(variant).toHaveTextContent('Original')

    fireEvent.click(variant)
    const offered = (await screen.findAllByRole('option')).map((option) => option.textContent)
    expect(offered).toEqual([
      en.fields.choiceNone,
      'Original',
      'Instrumental',
      'Slowed',
      'Sped up',
      'Remix',
    ])
    choose(screen.getByRole('option', { name: 'Sped up' }))

    await waitFor(() => expect(written).toEqual([{ meta: { variant: 'sped-up' } }]))
    // The box reads the answer by its label once it is kept.
    await waitFor(() =>
      expect(screen.getByRole('combobox', { name: 'Variant' })).toHaveTextContent('Sped up'),
    )
  })

  it('takes a choice back with the empty answer', async () => {
    const widget = await openFields(IDS.audio, 'Audio')

    fireEvent.click(within(widget).getByRole('combobox', { name: 'Variant' }))
    choose(await screen.findByRole('option', { name: en.fields.choiceNone }))

    await waitFor(() => expect(written).toEqual([{ meta: { variant: null } }]))
  })

  it('offers the next code under a numbered field nobody filled, and only there', async () => {
    // A profile that numbers its songs (ADR 0059): one song has its code,
    // the other was made before the numbering started.
    workspace.profile.config.work_meta_fields.unshift({
      key: 'code',
      label: 'Code',
      type: 'text',
      kinds: ['song'],
      numbered_from: 'CAT-001',
    })
    const song = workspace.works.find((w) => w.id === IDS.song)!
    song.meta = { ...song.meta, code: 'CAT-007' }

    const coded = await openFields(IDS.song, 'Song')
    expect(within(coded).getByRole('textbox', { name: 'Code' })).toHaveValue('CAT-007')
    expect(within(coded).queryByRole('button', { name: en.fields.giveCode })).toBeNull()
    cleanup()

    const widget = await openFields(IDS.draft, 'Song')
    fireEvent.click(within(widget).getByRole('button', { name: en.fields.giveCode }))

    await waitFor(() =>
      expect(within(widget).getByRole('textbox', { name: 'Code' })).toHaveValue('CAT-001'),
    )
    expect(within(widget).queryByRole('button', { name: en.fields.giveCode })).toBeNull()
    expect(written, 'given by the backend, not typed in').toEqual([])
  })

  it("draws no box for a field another kind's alone", async () => {
    const widget = await openFields(IDS.song, 'Song')

    // The fields every kind has are here; the audio's variant is not.
    expect(within(widget).getByRole('textbox', { name: 'BPM' })).toBeInTheDocument()
    expect(within(widget).queryByRole('combobox', { name: 'Variant' })).toBeNull()
    expect(within(widget).queryByText('Variant')).toBeNull()
  })
})
