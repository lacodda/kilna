import { Fragment, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { ProfileConfig, Scene, SceneNote, Work } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { checkStoryboard } from '@/lib/storyboard'
import {
  aboutKindsOf,
  chosenFrame,
  durationOf,
  framesByScene,
  ofKind,
  orderMoving,
  FRAME,
} from '@/lib/scenes'
import { allOf, useProfile, vocabularyOf, type Vocabulary } from '@/lib/useProfile'
import { EmptyState } from '@/components/ui/empty-state'
import { Panel } from '@/components/ui/panel'
import { SkeletonList } from '@/components/ui/skeleton'
import { Frame } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import type { RowAction } from '@/components/RowMenu'
import { ActionBar, actionsFor } from '@/features/assistant/ActionBar'
import { BoardFilters } from '@/features/work/tabs/scenes/BoardFilters'
import { BoardFoot } from '@/features/work/tabs/scenes/BoardFoot'
import { CONTEXT_ROLE, ContextStrip } from '@/features/work/tabs/scenes/ContextStrip'
import { EmptyBoard } from '@/features/work/tabs/scenes/EmptyBoard'
import { FrameViewer, type Viewing } from '@/features/work/tabs/scenes/FrameViewer'
import { SceneDetail } from '@/features/work/tabs/scenes/SceneDetail'
import { SceneRow } from '@/features/work/tabs/scenes/SceneRow'
import { columnsOf, SceneTable } from '@/features/work/tabs/scenes/SceneTable'
import { useSceneEdits } from '@/features/work/tabs/scenes/useSceneEdits'

interface Props {
  work: Work
}

/**
 * The storyboard: one row per scene, owned by the work.
 *
 * A scene is fields — a number, the part of the text it plays against, its
 * seconds, the kind of shot, a description — and prompt blocks keyed by the
 * profile, each edited on its own and copied on its own. The scaffold is
 * never typed into: the predecessor kept a scene as a markdown file edited
 * whole, and most of them were one save away from corruption.
 *
 * Laid out as the mockup draws it (v0.81): the assistant's actions for the
 * board, the filters and the shared context stand over it as three thin
 * lines; the board takes the height and scrolls inside; what is done to the
 * whole board stands at the foot, with what the board still owes as a chip at
 * its end. The context is a version of the `context` role — a body with
 * revisions, read by the assistant like any other text — and is edited where
 * versions are edited.
 */
export function ScenesTab({ work }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const vocabulary = boardVocabularyOf(profile.config, work.kind)
  const [shotType, setShotType] = useState<string | undefined>(undefined)
  // Which note the board is narrowed to: "every scene with her in it".
  const [withNote, setWithNote] = useState<string | undefined>(undefined)
  // Which frame the viewer is showing, if it is open.
  const [viewing, setViewing] = useState<Viewing | null>(null)

  // Which rows have their description and prompts open. Kept here rather
  // than in the row so opening one survives the board being re-read.
  const [opened, setOpened] = useState<ReadonlySet<string>>(new Set())
  const toggleOpen = (id: string) =>
    setOpened((open) => {
      const next = new Set(open)
      if (!next.delete(id)) next.add(id)
      return next
    })

  const scenes = useQuery(queries.scenes(work.id))
  // Who and where: what each scene is about. Read for the whole board in one
  // go rather than per row.
  const about = useQuery(queries.sceneNotes(work.id))
  // The pictures of the whole board, in one read for the same reason.
  const frames = useQuery(queries.sceneFrames(work.id))
  const framesForScene = framesByScene(frames.data ?? [])

  // The notes a scene can be about: the characters and the places, of the
  // kinds the profile names. Not every note in the workspace.
  const aboutKinds = aboutKindsOf(profile.config.note_kinds)
  const aboutShown = aboutKinds.length > 0
  const castable = useQuery({ ...queries.notesCastable(), enabled: aboutShown })
  const cast = (castable.data ?? []).filter((note) => aboutKinds.includes(note.kind))

  const edits = useSceneEdits(work.id)

  // A profile whose kinds name no kinds of shot and no blocks at all has no
  // storyboard to read a board in: the tab says where to give it one rather
  // than drawing an empty board.
  if (vocabulary.shot_types.length === 0 && vocabulary.scene_blocks.length === 0) {
    return (
      <Frame>
        <EmptyState plain title={t('scenes.noneForKind')} />
      </Frame>
    )
  }

  const all = scenes.data ?? []

  // What the board still owes. Computed from what this tab already holds —
  // the rows, the blocks the kind names, the pictures, the work's length —
  // rather than asked for: a second read would be a second opinion about the
  // same board, able to disagree with the rows beside it.
  const board = checkStoryboard(all, vocabulary.scene_blocks, framesForScene, durationOf(work))
  const counts = new Map<string, number>()
  for (const scene of all) {
    if (scene.shot_type !== null)
      counts.set(scene.shot_type, (counts.get(scene.shot_type) ?? 0) + 1)
  }

  const aboutByScene = new Map<string, SceneNote[]>()
  for (const link of about.data ?? []) {
    const held = aboutByScene.get(link.scene_id)
    if (held === undefined) aboutByScene.set(link.scene_id, [link])
    else held.push(link)
  }
  // The characters and places the board names, once each, for the filter.
  const named = new Map<string, SceneNote>()
  for (const link of about.data ?? []) {
    if (aboutKinds.includes(link.note_kind) && !named.has(link.note_id))
      named.set(link.note_id, link)
  }

  const byShot = shotType === undefined ? all : all.filter((scene) => scene.shot_type === shotType)
  const shown =
    withNote === undefined
      ? byShot
      : byShot.filter((scene) =>
          (aboutByScene.get(scene.id) ?? []).some((link) => link.note_id === withNote),
        )

  // The viewer walks the scenes that have a still to show, in board order.
  // Built from what is on screen, so a filtered board walks only what it shows
  // — the arrows never lead somewhere the person cannot see they are. Stills
  // only: the arrows compare pictures, and a scene with only a clip used to
  // open its clip as a picture.
  const stillsOf = (sceneId: string) => ofKind(framesForScene.get(sceneId) ?? [], FRAME)
  const withFrames = shown.filter((scene) => stillsOf(scene.id).length > 0)
  const viewingAt = viewing ? withFrames.findIndex((scene) => scene.id === viewing.sceneId) : -1
  const canStep = (direction: -1 | 1) =>
    viewingAt >= 0 && viewingAt + direction >= 0 && viewingAt + direction < withFrames.length
  const step = (direction: -1 | 1) => {
    if (!canStep(direction)) return
    const next = withFrames[viewingAt + direction]
    if (!next) return
    const stills = stillsOf(next.id)
    // The chosen frame is what the scene *is*; without one, its first.
    const frame = chosenFrame(stills) ?? stills[0]
    if (frame) setViewing({ sceneId: next.id, number: next.position, frame })
  }

  // A line of the board's check opens the scene it names. That scene may be
  // one the filters hide - and a click that did nothing then was a finding
  // of the audit - so the filters let go first, and the board scrolls to the
  // row once it is drawn.
  const go = (sceneId: string) => {
    if (!shown.some((scene) => scene.id === sceneId)) {
      setShotType(undefined)
      setWithNote(undefined)
    }
    // Opening the row is the whole gesture: the description and the prompts
    // are what a complaint is almost always about.
    setOpened((open) => new Set(open).add(sceneId))
    requestAnimationFrame(() =>
      document
        .getElementById(`scene-${sceneId}`)
        ?.scrollIntoView({ block: 'center', behavior: 'smooth' }),
    )
  }

  const moving = edits.renumber.isPending || edits.insert.isPending
  // While the board is being renumbered a second order would be computed
  // against numbers that are about to change; the menu's gestures wait.
  const unlessMoving = (act: () => void) => () => {
    if (!moving) act()
  }
  const actionsOf = (scene: Scene): RowAction[] => [
    {
      key: 'open',
      label: t(opened.has(scene.id) ? 'scenes.fold' : 'scenes.unfold', {
        number: scene.position,
      }),
      onSelect: () => toggleOpen(scene.id),
    },
    // A scene between two others, which until v0.72 meant adding one at the
    // end and typing its way back up the board.
    {
      key: 'before',
      label: t('scenes.insertBefore'),
      onSelect: unlessMoving(() => edits.insert.mutate({ scene, where: 'before' })),
    },
    {
      key: 'after',
      label: t('scenes.insertAfter'),
      onSelect: unlessMoving(() => edits.insert.mutate({ scene, where: 'after' })),
    },
    // A second try at the same moment, straight after it: the words and who
    // is in it, not the seconds or the pictures (`copyOf`).
    {
      key: 'copy',
      label: t('scenes.duplicate'),
      onSelect: unlessMoving(() =>
        edits.insert.mutate({ scene, where: 'after', copy: aboutByScene.get(scene.id) ?? [] }),
      ),
    },
    {
      key: 'delete',
      label: t('scenes.delete'),
      danger: true,
      onSelect: () => edits.remove.mutate(scene),
    },
  ]

  // Whether the profile has anything to offer here: the empty board is an
  // entrance to the actions when there are actions, and a plain board when
  // there are none.
  const hasActions = actionsFor(profile.config.prompts, work.kind, 'work').length > 0
  const shots = vocabulary.shot_types.length > 0

  return (
    <Frame
      head={
        <div className="flex w-full min-w-0 flex-col gap-2.5">
          {/* The profile's actions for this kind, on the board's own tab: the
              plot from the source, the board from the plot. A click starts a
              task and the answer comes back as a proposal, applied with one
              button; nothing here is written by the assistant itself. */}
          {hasActions && (
            <div className="flex flex-wrap items-center gap-2">
              <ActionBar workId={work.id} menu hint={t('scenes.actionsHint')} />
              <span className="ml-auto text-xs text-faint">{t('scenes.actionsHint')}</span>
            </div>
          )}
          <BoardFilters
            vocabulary={vocabulary}
            total={all.length}
            counts={counts}
            shotType={shotType}
            onShotType={setShotType}
            named={[...named.values()]}
            withNote={withNote}
            onWithNote={setWithNote}
          />
          {vocabulary.version_roles.some((role) => role.key === CONTEXT_ROLE) && (
            <ContextStrip workId={work.id} />
          )}
        </div>
      }
      foot={
        <BoardFoot
          work={work}
          scenes={all}
          framesForScene={framesForScene}
          board={board}
          onGo={go}
        />
      }
    >
      {scenes.data === undefined ? (
        <Panel className="flex min-h-0 flex-1 flex-col">
          <Loaded query={scenes} skeleton={<SkeletonList rows={4} secondary={false} />} plain>
            {() => null}
          </Loaded>
        </Panel>
      ) : all.length === 0 ? (
        <Panel className="flex min-h-0 flex-1 flex-col">
          <EmptyBoard work={work} hasActions={hasActions} />
        </Panel>
      ) : (
        <SceneTable
          shots={shots}
          about={aboutShown}
          none={shown.length === 0 ? t('scenes.noneOfShot') : null}
        >
          {(paneWidth) =>
            shown.map((scene) => (
              // Keyed on the scene alone. It used to carry `updated_at` too,
              // so a change from elsewhere redrew the fields - and so did the
              // row's own save: every edit remounted it, and the focus a Tab
              // had just moved to the next field fell to the page with
              // whatever was being typed there. The fields follow the stored
              // value themselves now (InlineField and `useFieldDraft`).
              <Fragment key={scene.id}>
                <SceneRow
                  scene={scene}
                  vocabulary={vocabulary}
                  aboutShown={aboutShown}
                  about={aboutByScene.get(scene.id) ?? []}
                  frames={framesForScene.get(scene.id) ?? []}
                  open={opened.has(scene.id)}
                  onToggle={() => toggleOpen(scene.id)}
                  saving={edits.patch.isPending && edits.patch.variables?.id === scene.id}
                  onPatch={(changes) => edits.patch.mutate({ id: scene.id, changes })}
                  onViewFrame={(frame) =>
                    setViewing({ sceneId: scene.id, number: scene.position, frame })
                  }
                  onMoveTo={(position) =>
                    edits.renumber.mutate(orderMoving(all, scene.id, position))
                  }
                  total={all.length}
                  moving={moving}
                  actions={actionsOf(scene)}
                />
                {opened.has(scene.id) && (
                  <SceneDetail
                    scene={scene}
                    workId={work.id}
                    vocabulary={vocabulary}
                    columns={columnsOf({ shots, about: aboutShown })}
                    paneWidth={paneWidth}
                    frames={framesForScene.get(scene.id) ?? []}
                    cast={cast}
                    about={aboutByScene.get(scene.id) ?? []}
                    onAttach={(noteId) => edits.attach.mutate({ sceneId: scene.id, noteId })}
                    onDetach={(noteId) => edits.detach.mutate({ sceneId: scene.id, noteId })}
                    onPatch={(changes) => edits.patch.mutate({ id: scene.id, changes })}
                    onViewFrame={(frame) =>
                      setViewing({ sceneId: scene.id, number: scene.position, frame })
                    }
                  />
                )}
              </Fragment>
            ))
          }
        </SceneTable>
      )}

      <FrameViewer
        viewing={viewing}
        onClose={() => setViewing(null)}
        onStep={step}
        canStep={canStep}
      />
    </Frame>
  )
}

/**
 * The words the board is read in: the kind's own, or - for a work that holds
 * scenes while its kind names no storyboard - every kind's.
 *
 * The card shows the Scenes tab for such a work because nothing written may
 * hide (`tabs.ts`): the owner's audio releases were videos once, and kept
 * their still-frame scenes when they became audio. A board drawn with no
 * kinds of shot and no blocks would show their rows with every prompt gone,
 * so the blocks and the shots are borrowed from whichever kinds have them
 * (`allOf`), by key - the key is what a scene stores its text under. The
 * rest of the vocabulary stays the kind's own.
 */
function boardVocabularyOf(config: ProfileConfig, kind: string): Vocabulary {
  const own = vocabularyOf(config, kind)
  if (own.shot_types.length > 0 || own.scene_blocks.length > 0) return own
  return {
    ...own,
    shot_types: allOf(config, 'shot_types'),
    scene_blocks: allOf(config, 'scene_blocks'),
  }
}
