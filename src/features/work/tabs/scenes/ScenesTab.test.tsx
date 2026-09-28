import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, screen, waitFor, within } from '@testing-library/react'
import { emit } from '@tauri-apps/api/event'
import type { NewScene, Scene, SceneFrame, SceneNote, ScenePatch } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio, type Studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The Scenes tab (v0.81): a board whose cells are edited where they stand,
 * whose rows keep what is done to them in one menu, and whose check is a chip
 * at the foot rather than a panel over it.
 */

const FRAMES: SceneFrame[] = ['f-first', 'f-second'].map((id, index) => ({
  id,
  scene_id: IDS.sceneOpen,
  asset_id: `a-${id}`,
  kind: 'frame',
  position: index + 1,
  is_selected: false,
  path: `C:/studio/files/${id}.png`,
  original_name: `${id}.png`,
  created_at: NOW,
}))

let backend: Backend
let workspace: Studio

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  workspace = studio()
  backend = mockBackend({
    ...answersFor(workspace),
    list_scene_frames: () => FRAMES,
    list_scene_notes: () =>
      [
        {
          id: 'sn-keeper',
          scene_id: IDS.sceneClose,
          note_id: IDS.character,
          note_kind: 'character',
          note_title: 'The lantern keeper',
          created_at: NOW,
        },
      ] satisfies SceneNote[],
    update_scene: ({ id, patch }) => ({
      ...workspace.scenes.find((scene) => scene.id === id)!,
      ...(patch as ScenePatch),
    }),
    create_scene: ({ scene }) =>
      ({
        ...workspace.scenes[0]!,
        ...(scene as NewScene),
        id: 'sc-copy',
        position: 3,
      }) as Scene,
    attach_scene_note: ({ sceneId, noteId }) => ({ scene_id: sceneId, note_id: noteId }),
    attach_scene_frame: ({ sceneId, kind }) => ({ ...FRAMES[0]!, scene_id: sceneId, kind }),
    renumber_scenes: () => workspace.scenes,
    reorder_scene_frames: () => FRAMES,
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

async function openBoard() {
  const { client, container } = renderApp(`/works/${IDS.video}/scenes`)
  // The first screen of a file is the slow one: the whole app loads cold.
  await screen.findAllByRole('textbox', { name: en.scenes.description })
  await settled(client)
  return { client, container }
}

/** Open a scene's row by its own toggle. */
async function unfold(number: number) {
  fireEvent.click(
    screen.getByRole('button', { name: en.scenes.unfold.replace('{{number}}', String(number)) }),
  )
}

describe('a cell', () => {
  it("edits a description's first line where it stands, and leaves the paragraphs under it", async () => {
    workspace.scenes[0]!.description = 'The lantern is lit.\n\nSmoke drifts over the water.'
    const { client } = await openBoard()

    const [cell] = screen.getAllByRole('textbox', { name: en.scenes.description })
    expect(cell).toHaveValue('The lantern is lit.')

    fireEvent.focus(cell!)
    fireEvent.change(cell!, { target: { value: 'A paper lantern is lit.' } })
    fireEvent.blur(cell!)
    await settled(client)

    // A single-line box cannot hold a line break; had the whole description
    // gone through it, the paragraphs would have been flattened and saved.
    expect(backend.argsOf('update_scene')).toEqual([
      {
        id: IDS.sceneOpen,
        patch: { description: 'A paper lantern is lit.\n\nSmoke drifts over the water.' },
      },
    ])
  })
})

describe("a row's menu", () => {
  it('holds what is done to the row, and a copy lands straight after the original', async () => {
    const { client } = await openBoard()

    fireEvent.click(
      screen.getByRole('button', { name: en.scenes.rowMenu.replace('{{number}}', '2') }),
    )
    const menu = await screen.findByRole('menu')
    expect(
      within(menu)
        .getAllByRole('menuitem')
        .map((item) => item.textContent),
    ).toEqual([
      en.scenes.unfold.replace('{{number}}', '2'),
      en.scenes.insertBefore,
      en.scenes.insertAfter,
      en.scenes.duplicate,
      en.scenes.delete,
    ])

    await act(async () =>
      fireEvent.click(within(menu).getByRole('menuitem', { name: en.scenes.duplicate })),
    )
    await settled(client)

    // The words and who is in it; not the seconds, which would overlap.
    expect(backend.argsOf('create_scene')).toEqual([
      {
        scene: {
          work_id: IDS.video,
          section: 'Chorus',
          shot_type: undefined,
          description: 'The lantern drifts under the bridge.',
          blocks: {},
        },
      },
    ])
    expect(backend.argsOf('attach_scene_note')).toEqual([
      { sceneId: 'sc-copy', noteId: IDS.character },
    ])
    expect(backend.argsOf('renumber_scenes')).toEqual([
      { workId: IDS.video, ids: [IDS.sceneOpen, IDS.sceneClose, 'sc-copy'] },
    ])
  })
})

describe('the check', () => {
  it('is a chip that opens what is missing, and a line opens the scene', async () => {
    const { container } = await openBoard()

    // Both scenes are half written, and the board runs 30 seconds of a
    // 3:24 work: three things, in one chip rather than a panel of lines.
    const chip = screen.getByRole('button', {
      name: en.scenes.check.chip.replace('{{number}}', '3'),
    })
    expect(container.querySelector(`[data-scene-detail="${IDS.sceneClose}"]`)).toBeNull()

    fireEvent.click(chip)
    const line = await screen.findByRole('button', {
      name: en.scenes.check.noPrompt.replace('{{number}}', '2'),
    })
    fireEvent.click(line)

    await waitFor(() =>
      expect(container.querySelector(`[data-scene-detail="${IDS.sceneClose}"]`)).not.toBeNull(),
    )
  })
})

describe('who a scene is about', () => {
  it('is chosen among the characters and the places, not every note', async () => {
    const { container } = await openBoard()
    await unfold(1)

    const detail = container.querySelector<HTMLElement>(`[data-scene-detail="${IDS.sceneOpen}"]`)!
    const cast = await within(detail).findByRole('group', { name: en.scenes.about })
    expect(within(cast).getByRole('button', { name: 'The lantern keeper' })).toBeVisible()
    // An idea in the notes is not somebody in the shot.
    expect(within(cast).queryByRole('button', { name: 'A song about tides' })).toBeNull()
  })
})

describe("a scene's pictures", () => {
  it('are put in order from the keyboard, the whole order travelling', async () => {
    const { client, container } = await openBoard()
    await unfold(1)

    const detail = container.querySelector<HTMLElement>(`[data-scene-detail="${IDS.sceneOpen}"]`)!
    const [first] = within(detail).getAllByRole('button', { name: en.scenes.chooseFrame })
    fireEvent.keyDown(first!, { key: 'ArrowDown', altKey: true })
    await settled(client)

    expect(backend.argsOf('reorder_scene_frames')).toEqual([
      { sceneId: IDS.sceneOpen, kind: 'frame', ids: ['f-second', 'f-first'] },
    ])
  })

  it('arrive only in the scene the file was dropped on, with two open', async () => {
    const { client, container } = await openBoard()
    await unfold(1)
    await unfold(2)
    await settled(client)

    // jsdom lays nothing out: only the stills of scene 2 are said to be
    // somewhere on screen, and the drop lands inside them.
    const target = container.querySelector(
      `[data-scene-detail="${IDS.sceneClose}"] [data-scene-strip="frame"]`,
    )!
    const box = { x: 0, y: 0, top: 0, left: 0, bottom: 100, right: 100, width: 100, height: 100 }
    const nowhere = { x: 0, y: 0, top: 0, left: 0, bottom: 0, right: 0, width: 0, height: 0 }
    vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(function (
      this: HTMLElement,
    ) {
      const at = this === target ? box : nowhere
      return { ...at, toJSON: () => at } as DOMRect
    })

    await act(async () =>
      emit('tauri://drag-drop', { paths: ['C:/in/lantern.png'], position: { x: 50, y: 50 } }),
    )
    await settled(client)

    expect(backend.argsOf('attach_scene_frame')).toEqual([
      { sceneId: IDS.sceneClose, kind: 'frame', source: 'C:/in/lantern.png' },
    ])
  })
})

describe('the package', () => {
  it('is offered only when there is something to pack', async () => {
    backend.answer('can_export_package', () => false)
    await openBoard()
    expect(screen.queryByRole('button', { name: en.scenes.package.action })).toBeNull()
    expect(backend.argsOf('can_export_package')).toEqual([{ workId: IDS.video }])
  })
})
