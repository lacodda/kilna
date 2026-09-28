import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, screen, within } from '@testing-library/react'
import type {
  Asset,
  GeneratedFields,
  Release,
  ReleaseFieldValue,
  ReleasePatch,
} from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The Releases tab (v0.80): a release is edited where it stands.
 *
 * Its row unrolls in place - the date, the link, what it goes out as, the
 * files that go with it - and saves as it goes, one field at a time. Text the
 * work would write over what someone typed is shown before it lands, and what
 * is kept is exactly what was shown.
 */

const FIELDS: ReleaseFieldValue[] = [
  { key: 'title', label: 'Title', type: 'line', value: 'Paper Lanterns', has_template: true },
  {
    key: 'description',
    label: 'Description',
    type: 'text',
    value: 'Typed by hand.',
    has_template: true,
  },
  { key: 'tags', label: 'Tags', type: 'tags', value: '', has_template: false },
]

let backend: Backend

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  const workspace = studio()
  backend = mockBackend({
    ...answersFor(workspace),
    release_fields: () => FIELDS,
    preview_release_fields: () =>
      ({
        values: { title: 'Paper Lanterns', description: 'Written from the lyrics.' },
        refused: [],
      }) satisfies GeneratedFields,
    set_release_fields: ({ id }) => workspace.releases.find((r) => r.id === id)!,
    generate_release_fields: () => {
      throw new Error('the text kept was not the text shown')
    },
    // Kept, as the backend keeps it: the list read back after the save is
    // what the form follows.
    update_release: ({ id, patch }) => {
      const release = workspace.releases.find((r) => r.id === id)!
      Object.assign(release, patch as ReleasePatch)
      return release satisfies Release
    },
    list_release_assets: ({ releaseId }) =>
      releaseId === IDS.youtube
        ? ([
            {
              id: 'a-thumb',
              profile_id: IDS.profile,
              work_id: null,
              release_id: IDS.youtube,
              kind: 'attachment',
              path: 'C:/studio/files/a-thumb.png',
              label: null,
              original_name: 'thumbnail.png',
              style_brick_id: null,
              created_at: NOW,
            },
          ] satisfies Asset[])
        : [],
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

/** The clip's Releases tab, with its one release unrolled. */
async function unrolled() {
  const { client } = renderApp(`/works/${IDS.video}/releases`)
  await settled(client)
  const main = await screen.findByRole('main')
  const row = await within(main).findByRole('button', { name: 'YouTube' })
  expect(row).toHaveAttribute('aria-expanded', 'false')

  fireEvent.click(row)
  await settled(client)
  expect(row).toHaveAttribute('aria-expanded', 'true')
  return { client, main }
}

describe('the releases tab', () => {
  it('unrolls a release in place: its date, its link, its fields and its files', async () => {
    const { main } = await unrolled()

    // No dialog: the form is in the row.
    expect(screen.queryByRole('dialog')).toBeNull()
    expect(within(main).getByRole('button', { name: en.calendar.slotDate })).toBeInTheDocument()
    expect(within(main).getByRole('textbox', { name: en.calendar.urlPrompt })).toHaveValue('')
    expect(within(main).getByRole('textbox', { name: 'Description' })).toHaveValue('Typed by hand.')
    expect(within(main).getByText('thumbnail.png')).toBeInTheDocument()
    expect(backend.argsOf('list_release_assets')).toEqual([{ releaseId: IDS.youtube }])
  })

  it('saves a link when it is left, and nothing else with it', async () => {
    const { client, main } = await unrolled()

    const link = within(main).getByRole('textbox', { name: en.calendar.urlPrompt })
    fireEvent.change(link, { target: { value: '  https://example.com/watch  ' } })
    await act(async () => {
      fireEvent.blur(link)
    })
    await settled(client)

    expect(backend.argsOf('update_release')).toEqual([
      { id: IDS.youtube, patch: { url: 'https://example.com/watch' } },
    ])

    // Left again untouched, it writes nothing a second time.
    await act(async () => {
      fireEvent.blur(link)
    })
    await settled(client)
    expect(backend.argsOf('update_release')).toHaveLength(1)
  })

  it('offers only the kinds its own work goes out as', async () => {
    const { main } = await unrolled()

    fireEvent.click(within(main).getByRole('combobox', { name: en.releases.kind }))
    const offered = (await screen.findAllByRole('option')).map((option) => option.textContent)
    // A video's two doors - not the song's audio release.
    expect(offered).toEqual(['YouTube', 'Premiere'])
  })

  it('shows what the work would write before it replaces what was typed', async () => {
    const { client, main } = await unrolled()

    await act(async () => {
      fireEvent.click(within(main).getByRole('button', { name: en.releases.meta.regenerate }))
    })
    await settled(client)

    // Shown, not written: only the field that would change, under its box,
    // and the box still holds what was typed.
    expect(backend.argsOf('preview_release_fields')).toEqual([{ id: IDS.youtube }])
    expect(backend.argsOf('set_release_fields')).toEqual([])
    expect(within(main).getByText('Written from the lyrics.')).toBeInTheDocument()
    expect(within(main).getAllByText(en.releases.meta.proposed)).toHaveLength(1)
    expect(within(main).getByRole('textbox', { name: 'Description' })).toHaveValue('Typed by hand.')

    await act(async () => {
      fireEvent.click(within(main).getByRole('button', { name: en.releases.meta.replace }))
    })
    await settled(client)

    // What is kept is the text that was shown, not a second rendering.
    expect(backend.argsOf('set_release_fields')).toEqual([
      { id: IDS.youtube, values: { description: 'Written from the lyrics.' } },
    ])
    expect(within(main).queryByText(en.releases.meta.proposed)).toBeNull()
  })

  it('keeps what was typed when the preview is turned down', async () => {
    const { client, main } = await unrolled()

    await act(async () => {
      fireEvent.click(within(main).getByRole('button', { name: en.releases.meta.regenerate }))
    })
    await settled(client)
    fireEvent.click(within(main).getByRole('button', { name: en.releases.meta.keepMine }))

    expect(within(main).queryByText('Written from the lyrics.')).toBeNull()
    expect(backend.argsOf('set_release_fields')).toEqual([])
  })
})
