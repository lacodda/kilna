import { useTranslation } from 'react-i18next'
import { useQueryClient } from '@tanstack/react-query'
import {
  attachSceneNote,
  createScene,
  deleteScene,
  detachSceneNote,
  listScenes,
  renumberScenes,
  updateScene,
} from '@/lib/api/scenes'
import type { Scene, SceneNote, ScenePatch } from '@/lib/api/types'
import { announceEdited } from '@/lib/edited'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { copyOf, orderMoving } from '@/lib/scenes'
import { say } from '@/lib/toast'
import { announceDeleted } from '@/lib/trash'

/** Where a new scene lands beside the one it was asked for from. */
export type Beside = 'before' | 'after'

/**
 * Everything done to one row of the board, for the whole board.
 *
 * One set of mutations rather than one per row: a row is drawn fifty times,
 * and what the board has to know - which row is saving, whether the order is
 * moving - is a question about these, asked once.
 */
export function useSceneEdits(workId: string) {
  const { t } = useTranslation()
  const client = useQueryClient()

  // A refused write leaves the fields showing what was typed; the board is
  // read again so it shows what is actually stored.
  const reread = () => {
    for (const key of refresh.scene) void client.invalidateQueries({ queryKey: key })
  }

  const patch = useAppMutation({
    mutationFn: ({ id, changes }: { id: string; changes: ScenePatch }) => updateScene(id, changes),
    onSuccess: () => {
      announceEdited({ client, message: t('toast.sceneEdited'), refresh: refresh.scene })
    },
    onError: (cause) => {
      reread()
      say.failedTo(t('toast.sceneSaveFailed'), cause)
    },
  })

  const attach = useAppMutation({
    mutationFn: ({ sceneId, noteId }: { sceneId: string; noteId: string }) =>
      attachSceneNote(sceneId, noteId),
    failure: 'toast.sceneSaveFailed',
    refresh: refresh.scene,
  })

  const detach = useAppMutation({
    mutationFn: ({ sceneId, noteId }: { sceneId: string; noteId: string }) =>
      detachSceneNote(sceneId, noteId),
    failure: 'toast.sceneSaveFailed',
    refresh: refresh.scene,
  })

  // The board's order, set from a list: the number typed on a scene lands
  // here, and so does every scene added beside another.
  const renumber = useAppMutation({
    mutationFn: (ids: string[]) => renumberScenes(workId, ids),
    refresh: refresh.scene,
    onSuccess: () => say.ok(t('scenes.renumbered')),
    onError: (cause) => {
      // Re-read so the numbers on screen are the stored ones: a refused
      // renumbering left them exactly as they were.
      reread()
      say.failedTo(t('scenes.renumberFailed'), cause)
    },
  })

  // A scene beside another - empty, or a copy of it. It is created at the
  // end - the only place a new row can go before the board knows about it -
  // and the board is then numbered with it in the place asked for (a copy
  // names its people and places in between). Several steps rather than one
  // because the scene has no id until it exists, and the order is a list of
  // ids; the person sees one gesture because each step follows the last
  // without asking.
  const insert = useAppMutation({
    mutationFn: async ({
      scene,
      where,
      copy,
    }: {
      scene: Scene
      where: Beside
      /** Who the copy is about, when it is a copy: it names them too. */
      copy?: SceneNote[]
    }) => {
      // The board as the backend has it, not as the screen filtered it: the
      // order names every scene, and a list built from a narrowed view would
      // be refused for the ones it left out — rightly.
      const board = await listScenes(workId)
      const created = await createScene(copy === undefined ? { work_id: workId } : copyOf(scene))
      for (const link of copy ?? []) await attachSceneNote(created.id, link.note_id)
      // Its place counted on that board, so a number the screen had not yet
      // caught up with cannot send the new row somewhere else.
      const at = board.findIndex((held) => held.id === scene.id) + 1 || scene.position
      const to = where === 'before' ? at : at + 1
      await renumberScenes(workId, orderMoving([...board, created], created.id, to))
      return to
    },
    refresh: refresh.scene,
    onSuccess: (number, { scene, copy }) => {
      say.ok(
        copy === undefined
          ? t('scenes.added', { number })
          : t('scenes.duplicated', { number, from: scene.position }),
      )
    },
    onError: (cause) => {
      reread()
      say.failedTo(t('toast.sceneSaveFailed'), cause)
    },
  })

  const remove = useAppMutation({
    mutationFn: (scene: Scene) => deleteScene(scene.id),
    failure: 'toast.sceneSaveFailed',
    onSuccess: (deletionId, scene) =>
      announceDeleted({
        client,
        deletionId,
        message: t('toast.sceneDeleted', { number: scene.position }),
        refresh: refresh.scene,
      }),
  })

  return { patch, attach, detach, renumber, insert, remove }
}
