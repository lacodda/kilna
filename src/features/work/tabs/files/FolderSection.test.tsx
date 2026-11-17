import { beforeEach, describe, expect, it } from 'vitest'
import { fireEvent, screen, waitFor, within } from '@testing-library/react'
import type { FolderFile, WorkFolder } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

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
  file: { state: 'noFolder' },
}

let backend: Backend
let folder: WorkFolder

beforeEach(() => {
  // The look is the person's, kept on this machine: each test starts from
  // the default one.
  localStorage.clear()
  folder = FOUND
  backend = mockBackend({
    ...answersFor(studio()),
    work_folder: () => folder,
    open_work_folder: () => {
      folder = { ...FOUND, files: [] }
      return folder
    },
    open_media: () => null,
    attach_asset: () => ({}),
  })
})

describe("a work's folder on disk", () => {
  it('gives a song a Files tab, and lists its folder by type or by where each file lies', async () => {
    renderApp(`/works/${IDS.song}/files`)
    const section = await screen.findByRole('region', { name: en.folder.heading })
    expect(await within(section).findByText(FOLDER)).toBeVisible()
    const headings = () =>
      within(section)
        .getAllByRole('heading', { level: 3 })
        .map((heading) => heading.textContent)
    // By what they are (FR-14): pictures, sound, clips, the rest - every
    // folder below read, a file's own name under each.
    expect(headings()).toEqual([
      `${en.files.category.image} · 1`,
      `${en.files.category.audio} · 2`,
      `${en.files.category.video} · 1`,
      `${en.files.category.other} · 1`,
    ])
    expect(within(section).getByText('mix-final.wav')).toBeVisible()

    // Or by where they lie: the folder's own files first, then each folder
    // by its name.
    const bar = screen.getByRole('toolbar', { name: en.files.arrange })
    fireEvent.click(within(bar).getByRole('radio', { name: en.files.groupBy.place }))
    expect(headings()).toEqual([`${en.folder.top} · 2`, 'audio-release/v1 · 2', 'clip/v1 · 1'])

    // Narrowed to one category, the others leave no heading behind.
    fireEvent.click(within(bar).getByRole('button', { name: new RegExp(en.files.category.audio) }))
    expect(headings()).toEqual(['audio-release/v1 · 2'])
    fireEvent.click(within(bar).getByRole('button', { name: new RegExp(en.files.category.audio) }))

    // As a table: the size and the type beside each name.
    fireEvent.click(within(bar).getByRole('radio', { name: en.files.view.table }))
    const row = within(section).getByText('still.png').closest('tr')!
    expect(within(row).getByText(en.files.category.image)).toBeVisible()
    expect(within(row).getByText(/1\s?kB/)).toBeVisible()
    expect(
      within(section).getAllByRole('columnheader', { name: en.files.column.size }).length,
    ).toBeGreaterThan(0)

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

  it('says a folder is not there yet, and makes it by opening it', async () => {
    folder = { ...FOUND, state: 'absent', files: [] }
    const { client } = renderApp(`/works/${IDS.song}/files`)
    const section = await screen.findByRole('region', { name: en.folder.heading })
    expect(await within(section).findByText(en.folder.absent)).toBeVisible()
    await settled(client)

    // No "make the folder" to press first (FR-04): the folder is made when
    // it is first needed, and opening it is needing it.
    expect(within(section).queryByRole('button', { name: /make the folder/i })).toBeNull()
    fireEvent.click(within(section).getByRole('button', { name: en.folder.openFolder }))
    await waitFor(() => expect(backend.argsOf('open_work_folder')).toEqual([{ workId: IDS.song }]))
    expect(await within(section).findByText(en.folder.empty)).toBeVisible()
  })

  it('sends a workspace with no media folder to choose one', async () => {
    folder = { state: 'noRoot', files: [], truncated: false, file: { state: 'noFolder' } }
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
      file: { state: 'noFolder' },
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

describe("a work's finished file", () => {
  const take = `${FOLDER}\\clip\\v1\\take-2.mp4`

  it('says which file the rule found, marks its tile, and lets another be chosen', async () => {
    folder = {
      ...FOUND,
      file: { state: 'found', rule: 'clip/*/*.mp4', relative: 'clip/v1/take-2.mp4', path: take },
    }
    backend = mockBackend({
      ...answersFor(studio()),
      work_folder: () => folder,
      update_work: ({ id }) => studio().works.find((one) => one.id === id)!,
    })
    const { client } = renderApp(`/works/${IDS.song}/files`)
    const section = await screen.findByRole('region', { name: en.folder.heading })
    expect(
      await within(section).findByText(
        en.folder.finished.found
          .replace('{{file}}', 'clip/v1/take-2.mp4')
          .replace('{{rule}}', 'clip/*/*.mp4'),
      ),
    ).toBeVisible()
    await settled(client)

    const marked = within(section).getByText('take-2.mp4').closest('li')!
    expect(within(marked).getByLabelText(en.files.role.finished)).toBeVisible()
    expect(
      within(marked).queryByRole('button', { name: en.folder.finished.makeIt }),
      'the finished file is not offered to itself',
    ).toBeNull()
    const picture = within(section).getByText('still.png').closest('li')!
    expect(
      within(picture).queryByRole('button', { name: en.folder.finished.makeIt }),
      'a still is not what a work is finished as',
    ).toBeNull()

    const rough = within(section).getByText('mix-rough.wav').closest('li')!
    fireEvent.click(within(rough).getByRole('button', { name: en.folder.finished.makeIt }))
    await waitFor(() =>
      expect(backend.argsOf('update_work')).toEqual([
        { id: IDS.song, patch: { file: 'audio-release/v1/mix-rough.wav' } },
      ]),
    )
  })

  it('says a chosen file is gone, and gives the choice back to the rule', async () => {
    folder = {
      ...FOUND,
      file: { state: 'chosenMissing', rule: 'clip/*/*.mp4', relative: 'clip/v1/take-1.mp4' },
    }
    backend = mockBackend({
      ...answersFor(studio()),
      work_folder: () => folder,
      update_work: ({ id }) => studio().works.find((one) => one.id === id)!,
    })
    const { client } = renderApp(`/works/${IDS.song}/files`)
    const section = await screen.findByRole('region', { name: en.folder.heading })
    expect(
      await within(section).findByText(
        en.folder.finished.chosenMissing.replace('{{file}}', 'clip/v1/take-1.mp4'),
      ),
    ).toBeVisible()
    await settled(client)

    fireEvent.click(within(section).getByRole('button', { name: en.folder.finished.backToRule }))
    await waitFor(() =>
      expect(backend.argsOf('update_work')).toEqual([{ id: IDS.song, patch: { file: null } }]),
    )
  })
})
