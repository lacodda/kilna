import { beforeEach, describe, expect, it } from 'vitest'
import { fireEvent, screen, waitFor, within } from '@testing-library/react'
import type { FolderFile, WorkFolder } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'
import { byFolder } from './FolderSection'

/*
 * A work's folder on disk (v0.93, ADR 0057): found by the name its kind gives
 * it under the media folder, looked at and never owned. The song is the case
 * that matters - it goes out only as what is made from it, so until now it
 * had no Files tab at all, and its folder is where its takes and stills live.
 */

const ROOT = 'D:\\media'
const FOLDER = `${ROOT}\\songs\\Harbour lights`

function file(relative: string, modified = NOW): FolderFile {
  return {
    path: `${FOLDER}\\${relative.replaceAll('/', '\\')}`,
    relative,
    size: 1024,
    modified,
  }
}

const FOUND: WorkFolder = {
  state: 'found',
  root: ROOT,
  path: FOLDER,
  template: 'songs/{origin.title}',
  files: [
    file('audio-release/v1/mix-final.wav', '2026-09-20T10:00:00Z'),
    file('audio-release/v1/mix-rough.wav', '2026-09-10T10:00:00Z'),
    file('clip/v1/take-2.mp4'),
    file('notes.md'),
    file('still.png'),
  ],
  truncated: false,
}

let backend: Backend
let folder: WorkFolder

beforeEach(() => {
  folder = FOUND
  backend = mockBackend({
    ...answersFor(studio()),
    work_folder: () => folder,
    create_work_folder: () => {
      folder = { ...FOUND, files: [] }
      return folder
    },
    open_media: () => null,
    attach_asset: () => ({}),
  })
})

describe("a work's folder on disk", () => {
  it('gives a song a Files tab, and lists its folder grouped by where each file lies', async () => {
    renderApp(`/works/${IDS.song}/files`)
    const section = await screen.findByRole('region', { name: en.folder.heading })
    expect(await within(section).findByText(FOLDER)).toBeVisible()
    // The folder's own files first, then each folder by its name.
    const headings = within(section)
      .getAllByRole('heading', { level: 4 })
      .map((heading) => heading.textContent)
    expect(headings).toEqual([`${en.folder.top} · 2`, 'audio-release/v1 · 2', 'clip/v1 · 1'])
    expect(within(section).getByText('mix-final.wav')).toBeVisible()
    // A song sets no cover: it goes out as what is made from it.
    expect(screen.queryByRole('button', { name: en.files.setCover })).toBeNull()
  })

  it('opens a file where the system opens it, and copies one in only when asked', async () => {
    const { client } = renderApp(`/works/${IDS.song}/files`)
    const section = await screen.findByRole('region', { name: en.folder.heading })
    await within(section).findByText('still.png')
    await settled(client)

    fireEvent.click(
      within(section).getByRole('button', {
        name: en.folder.open.replace('{{name}}', 'still.png'),
      }),
    )
    await waitFor(() =>
      expect(backend.argsOf('open_media')).toEqual([{ path: `${FOLDER}\\still.png` }]),
    )
    expect(backend.argsOf('attach_asset')).toEqual([])

    const tile = within(section).getByText('still.png').closest('li')!
    fireEvent.click(within(tile).getByRole('button', { name: en.folder.copyIn }))
    await waitFor(() =>
      expect(backend.argsOf('attach_asset')).toEqual([
        { source: `${FOLDER}\\still.png`, asset: { work_id: IDS.song } },
      ]),
    )
  })

  it('says why there is nothing to show, and makes a missing folder when asked', async () => {
    folder = { ...FOUND, state: 'absent', files: [] }
    const { client } = renderApp(`/works/${IDS.song}/files`)
    const section = await screen.findByRole('region', { name: en.folder.heading })
    expect(await within(section).findByText(en.folder.absent)).toBeVisible()
    await settled(client)

    fireEvent.click(within(section).getByRole('button', { name: en.folder.create }))
    await waitFor(() =>
      expect(backend.argsOf('create_work_folder')).toEqual([{ workId: IDS.song }]),
    )
    expect(await within(section).findByText(en.folder.empty)).toBeVisible()
  })

  it('sends a workspace with no media folder to choose one', async () => {
    folder = { state: 'noRoot', files: [], truncated: false }
    renderApp(`/works/${IDS.song}/files`)
    const section = await screen.findByRole('region', { name: en.folder.heading })
    expect(await within(section).findByText(en.folder.noRoot, { exact: false })).toBeVisible()
    expect(within(section).getByRole('button', { name: en.folder.chooseRoot })).toBeVisible()
  })

  it('says which field a folder name waits for', async () => {
    folder = {
      state: 'unfilled',
      root: ROOT,
      template: 'songs/{code}',
      unfilled: 'code',
      files: [],
      truncated: false,
    }
    renderApp(`/works/${IDS.song}/files`)
    const section = await screen.findByRole('region', { name: en.folder.heading })
    expect(
      await within(section).findByText(
        en.folder.unfilled
          .replace('{{template}}', 'songs/{code}')
          .replace('{{placeholder}}', 'code'),
      ),
    ).toBeVisible()
  })
})

describe("the card's player", () => {
  it('plays the newest sound of the work, and a sound tile asks it to', async () => {
    renderApp(`/works/${IDS.song}/files`)
    const section = await screen.findByRole('region', { name: en.folder.heading })
    await within(section).findByText('mix-rough.wav')

    // In the header, over every tab: the newest take first. The tile of the
    // same file is a button of the same name; the header's is the bar's.
    const inHeader = async (name: string) =>
      (
        await screen.findAllByRole('button', { name: en.player.play.replace('{{name}}', name) })
      ).filter((button) => button.closest('header') !== null)
    expect(await inHeader('mix-final.wav')).toHaveLength(1)

    // A sound's tile chooses what the bar plays rather than opening a program.
    fireEvent.click(
      within(section).getByRole('button', {
        name: en.player.play.replace('{{name}}', 'mix-rough.wav'),
      }),
    )
    await waitFor(async () => expect(await inHeader('mix-rough.wav')).toHaveLength(1))
    expect(backend.argsOf('open_media')).toEqual([])
  })

  it('stays away from a work with nothing to play', async () => {
    folder = { ...FOUND, files: [file('still.png')] }
    renderApp(`/works/${IDS.song}/files`)
    const section = await screen.findByRole('region', { name: en.folder.heading })
    await within(section).findByText('still.png')
    expect(screen.queryByRole('button', { name: en.player.pause })).toBeNull()
    expect(
      screen.queryByRole('button', { name: new RegExp(en.player.play.split('“')[0]!) }),
    ).toBeNull()
  })
})

describe('the folder grouping', () => {
  it('puts the folder itself first and keeps the order within a group', () => {
    const groups = byFolder([file('b/2.png'), file('a/1.png'), file('top.png'), file('b/1.png')])
    expect(groups.map((group) => group.folder)).toEqual(['', 'a', 'b'])
    expect(groups[2]!.files.map((one) => one.relative)).toEqual(['b/2.png', 'b/1.png'])
  })
})
