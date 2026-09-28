import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { useNavigate } from 'react-router'
import { open, save } from '@tauri-apps/plugin-dialog'
import { exportPackage, writeTextFile } from '@/lib/api/data'
import { createScene, timeScenes } from '@/lib/api/scenes'
import type { Scene, SceneFrame, Work } from '@/lib/api/types'
import { cloneWork } from '@/lib/api/works'
import { montageFileName, montageList } from '@/lib/montage'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import type { Storyboard } from '@/lib/storyboard'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { PromptDialog } from '@/components/AppDialog'
import { StoryboardCheck } from '@/features/work/tabs/scenes/StoryboardCheck'

interface Props {
  work: Work
  /** The whole board, never the filtered view: see the montage list. */
  scenes: Scene[]
  framesForScene: Map<string, SceneFrame[]>
  /** What the board still owes. */
  board: Storyboard
  /** Take me to this scene, from a line of what the board owes. */
  onGo: (sceneId: string) => void
}

/**
 * What is done to the board as a whole, at the tab's foot: it followed the
 * last of fifty scenes down, and adding the fifty-first meant scrolling to
 * find the button. At the far end, the chip that says how much the board
 * still owes - the mockup's foot, where the eye lands after the last row.
 *
 * Words without glyphs, as the mockup draws them: six buttons with an icon
 * each broke onto a second line in the default window and took a row of
 * scenes with them.
 */
export function BoardFoot({ work, scenes, framesForScene, board, onGo }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const [cloning, setCloning] = useState(false)

  // Whether there is anything to pack. Asked, not assumed from the rows: a
  // board with no scenes can still have a release whose words are written,
  // and a folder of those is worth offering; a song with neither is not
  // offered a folder holding one nearly empty page.
  const packable = useQuery(queries.canExportPackage(work.id))

  const add = useAppMutation({
    mutationFn: () => createScene({ work_id: work.id }),
    failure: 'toast.sceneSaveFailed',
    refresh: refresh.scene,
    onSuccess: (created) => say.ok(t('scenes.added', { number: created.position })),
  })

  // The board's first timing: the work's length divided between the scenes,
  // dragged by hand from there. Refused when the work has no length, and the
  // refusal says where to give it one.
  const time = useAppMutation({
    mutationFn: () => timeScenes(work.id),
    failure: 'toast.sceneTimeFailed',
    refresh: refresh.scene,
    onSuccess: (timed) => say.ok(t('scenes.timed', { count: timed.length })),
  })

  const clone = useAppMutation({
    mutationFn: (title: string) => cloneWork(work.id, title),
    failure: 'scenes.clone.action',
    // A work made, not a scene changed: the board's own areas are untouched,
    // and only the list every work appears in gains a row.
    refresh: [keys.works],
    onSuccess: (made) => {
      setCloning(false)
      say.ok(t('scenes.clone.done', { title: made.work.title, count: made.scenes }))
      // Straight into the copy: the whole point is to start changing it, and
      // leaving the person on the original is a click they did not ask for.
      void navigate(`/works/${made.work.id}/scenes`)
    },
  })

  // The list is rendered from the WHOLE board, never from the filtered view:
  // a cut is the video end to end, and handing an editor the four scenes that
  // happened to match a filter would be a list that silently omits the rest.
  const montage = () =>
    montageList(scenes, framesForScene, { missing: t('scenes.montage.missing') })

  const copyMontage = () => {
    navigator.clipboard.writeText(montage()).then(
      () => say.ok(t('scenes.montage.copied')),
      (cause: unknown) => say.failedTo(t('scenes.montage.copy'), cause),
    )
  }

  const saveMontage = async () => {
    try {
      const path = await save({
        defaultPath: montageFileName(work.title),
        filters: [{ name: 'Text', extensions: ['txt'] }],
      })
      if (typeof path !== 'string') return
      await writeTextFile(path, montage())
      say.ok(t('scenes.montage.saved'))
    } catch (cause) {
      say.failedTo(t('scenes.montage.save'), cause)
    }
  }

  // The whole thing in a folder: the board with every prompt, the pictures
  // under names that say what they are, and what the releases go out as. The
  // folder is made inside the one chosen, named after the work, so choosing
  // the same parent twice does not put two works in one heap.
  const savePackage = async () => {
    try {
      const directory = await open({ directory: true, title: t('scenes.package.title') })
      if (typeof directory !== 'string') return
      const report = await exportPackage(work.id, directory)
      say.ok(
        t('scenes.package.done', {
          scenes: report.scenes,
          files: report.files,
          path: report.directory,
        }),
      )
      // Said separately, and only when there is something to say: a package
      // is also how someone finds out what the board is still missing.
      if (report.withoutMaterial > 0) {
        say.warn(t('scenes.package.withoutMaterial', { count: report.withoutMaterial }))
      }
    } catch (cause) {
      say.failedTo(t('scenes.package.failed'), cause)
    }
  }

  const any = scenes.length > 0

  return (
    <>
      <Button size="sm" disabled={add.isPending} onClick={() => add.mutate()}>
        {t('scenes.add')}
      </Button>
      {any && (
        <Button
          size="sm"
          disabled={time.isPending}
          onClick={() => time.mutate()}
          title={t('scenes.timeHint')}
        >
          {t('scenes.time')}
        </Button>
      )}

      {/* What the board is cut from, as text: the same list to the
          clipboard or to a file, because one of them is at hand and the
          other survives the next copy. */}
      {any && (
        <>
          <Button size="sm" onClick={copyMontage}>
            {t('scenes.montage.copy')}
          </Button>
          <Button size="sm" onClick={() => void saveMontage()}>
            {t('scenes.montage.save')}
          </Button>
        </>
      )}
      {/* The montage list is what one program needs; this is what a person
          needs: everything at once, in a folder, readable without kilna. */}
      {packable.data === true && (
        <Button size="sm" title={t('scenes.package.hint')} onClick={() => void savePackage()}>
          {t('scenes.package.action')}
        </Button>
      )}
      {/* Not a version of the video: a second work from the same donor, with
          this board copied into it. The first stays as it is, which is the
          point — the two get compared. */}
      {any && (
        <Button
          size="sm"
          disabled={clone.isPending}
          title={t('scenes.clone.hint')}
          onClick={() => setCloning(true)}
        >
          {t('scenes.clone.action')}
        </Button>
      )}

      <span className="ml-auto">
        <StoryboardCheck board={board} onGo={onGo} />
      </span>

      <PromptDialog
        open={cloning}
        onOpenChange={setCloning}
        title={t('scenes.clone.title')}
        description={t('scenes.clone.hint')}
        label={t('scenes.clone.title')}
        initialValue={t('scenes.clone.suffix', { title: work.title })}
        confirmLabel={t('scenes.clone.action')}
        onSubmit={(title) => clone.mutate(title)}
      />
    </>
  )
}
