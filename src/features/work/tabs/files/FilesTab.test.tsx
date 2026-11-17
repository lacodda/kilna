import { beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, screen, waitFor, within } from '@testing-library/react'
import { open } from '@tauri-apps/plugin-dialog'
import type { Asset } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The Files tab: a grid of tiles, the cover tagged, and a removal that asks
 * first - the one removal in the card nothing brings back. On the clip: a
 * song has no Files tab since v0.86, going out only as what is made from it.
 */

const COVER: Asset = {
  trial_id: null,
  id: 'a-cover',
  profile_id: IDS.profile,
  work_id: IDS.video,
  release_id: null,
  kind: 'cover',
  path: 'C:/ws/media/a-cover.png',
  label: null,
  original_name: 'lanterns-cover.png',
  style_brick_id: null,
  note_id: null,
  canon_fact_id: null,
  created_at: NOW,
  size_bytes: 1024,
  modified_at: NOW,
}

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }))

/** What the window hears of files held over it and dropped on it. */
type Dropped = (event: { payload: { type: string; paths: string[] } }) => void
let drop: Dropped = () => {}
vi.mock('@tauri-apps/api/webview', () => ({
  getCurrentWebview: () => ({
    onDragDropEvent: (heard: Dropped) => {
      drop = heard
      return Promise.resolve(() => {})
    },
  }),
}))

let backend: Backend
/** What stands attached to the clip. */
let attached: Asset[]

beforeEach(() => {
  localStorage.clear()
  vi.mocked(open).mockReset()
  attached = [COVER]
  backend = mockBackend({
    ...answersFor(studio()),
    list_work_assets: ({ workId }) => (workId === IDS.video ? attached : []),
    detach_asset: () => null,
    attach_asset: ({ source }) => {
      const name = String(source).split('/').pop()!
      const asset: Asset = {
        ...COVER,
        id: `a-${name}`,
        kind: 'attachment',
        path: `C:/ws/media/a-${name}`,
        original_name: name,
        size_bytes: 2048,
      }
      attached = [...attached, asset]
      return asset
    },
  })
})

describe('a file on the card', () => {
  it('is tagged as the cover when it is one', async () => {
    renderApp(`/works/${IDS.video}/files`)
    const tile = (await screen.findByText('lanterns-cover.png')).closest('li')!
    expect(within(tile).getByText(en.files.cover)).toBeVisible()
  })

  it('is removed only once the question is answered', async () => {
    const { client } = renderApp(`/works/${IDS.video}/files`)
    await screen.findByText('lanterns-cover.png')
    await settled(client)

    fireEvent.click(screen.getByRole('button', { name: en.files.detach }))
    const question = await screen.findByRole('alertdialog')
    expect(
      within(question).getByText(en.files.detachTitle.replace('{{name}}', 'lanterns-cover.png')),
    ).toBeVisible()
    expect(backend.argsOf('detach_asset')).toEqual([])

    fireEvent.click(within(question).getByRole('button', { name: en.files.detachConfirm }))
    await waitFor(() => expect(backend.argsOf('detach_asset')).toEqual([{ id: COVER.id }]))
  })

  it('can be any file, and several are attached at once (FR-14)', async () => {
    vi.mocked(open).mockResolvedValue(['D:/in/lyrics.txt', 'D:/in/subs.srt', 'D:/in/stems.zip'])
    const { client } = renderApp(`/works/${IDS.video}/files`)
    await screen.findByText('lanterns-cover.png')
    await settled(client)

    fireEvent.click(screen.getByRole('button', { name: en.files.attach }))
    await waitFor(() => expect(backend.argsOf('attach_asset')).toHaveLength(3))

    // Asked for with no filter and for as many as are picked - the cover is
    // the one thing that has to be a picture.
    expect(vi.mocked(open).mock.calls[0]![0]).toEqual({ multiple: true })
    expect(backend.argsOf('attach_asset').map((call) => call.source)).toEqual([
      'D:/in/lyrics.txt',
      'D:/in/subs.srt',
      'D:/in/stems.zip',
    ])
    // One word for the lot.
    expect(
      await screen.findByText(en.files.attachedMany_other.replace('{{count}}', '3')),
    ).toBeVisible()
    // Filed by what they are: none of the three is a picture.
    const other = (await screen.findByRole('heading', { name: /^Other · 3$/ })).closest('section')!
    expect(within(other).getByText('subs.srt')).toBeVisible()

    fireEvent.click(screen.getByRole('button', { name: en.files.changeCover }))
    await waitFor(() => expect(vi.mocked(open)).toHaveBeenCalledTimes(2))
    expect(vi.mocked(open).mock.calls[1]![0]).toMatchObject({ multiple: false })
    expect(vi.mocked(open).mock.calls[1]![0]).toHaveProperty('filters')
  })

  it('is dropped onto a place drawn for it when nothing is attached yet', async () => {
    attached = []
    const { client } = renderApp(`/works/${IDS.video}/files`)
    expect(await screen.findByText(en.files.dropZone)).toBeVisible()
    await settled(client)

    // Held over the window, the place says so.
    act(() => drop({ payload: { type: 'over', paths: [] } }))
    expect(screen.getAllByText(en.files.dropHere).length).toBeGreaterThan(0)

    act(() => drop({ payload: { type: 'drop', paths: ['D:/in/take.mov', 'D:/in/mix.flac'] } }))
    await waitFor(() => expect(backend.argsOf('attach_asset')).toHaveLength(2))
    expect(
      await screen.findByText(en.files.attachedMany_other.replace('{{count}}', '2')),
    ).toBeVisible()
    expect(await screen.findByText('take.mov')).toBeVisible()
    expect(screen.queryByText(en.files.dropZone)).toBeNull()
  })
})
