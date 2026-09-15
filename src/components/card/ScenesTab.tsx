import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from 'react-router'
import { ChevronRight, Clock, Copy, Plus, Rows3, Trash2 } from 'lucide-react'
import {
  attachSceneNote,
  createScene,
  deleteScene,
  detachSceneNote,
  fileSrc,
  frameScenes,
  getVersion,
  listLinks,
  listNotes,
  listSceneFrames,
  listSceneNotes,
  listScenes,
  listVersions,
  timeScenes,
  updateScene,
  type Note,
  type Scene,
  type SceneBlock,
  type SceneFrame,
  type SceneNote,
  type ScenePatch,
  type Work,
} from '@/lib/api'
import { announceEdited } from '@/lib/edited'
import { keys } from '@/lib/query'
import { formatSeconds, parseTimecode } from '@/lib/timecode'
import { chosenFrame, framesByScene, readinessOf, type Readiness } from '@/lib/scenes'
import { say } from '@/lib/toast'
import { announceDeleted } from '@/lib/trash'
import { useProfile, vocabularyOf, type Vocabulary } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Markdown } from '@/components/ui/Markdown'
import { Panel } from '@/components/ui/panel'
import { Select } from '@/components/ui/AppSelect'
import { Skeleton } from '@/components/ui/Skeleton'
import { Textarea } from '@/components/ui/textarea'
import { ActionBar, actionsFor } from '@/components/assistant/ActionBar'
import { FrameViewer, type Viewing } from '@/components/card/FrameViewer'
import { SceneFrames } from '@/components/card/SceneFrames'

interface Props {
  work: Work
}

/** The role that holds what every scene has in common: hero, palette, lens. */
const CONTEXT_ROLE = 'context'

/** The queries a scene changes: the board, and the feed. */
/** The queries a scene changes: the board (its rows and what they are
    about, which share the prefix), and the feed. */
const REFRESHED = [keys.scenes, keys.journal] as const

/**
 * The storyboard: one row per scene, owned by the work.
 *
 * A scene is fields — a number, the part of the text it plays against, its
 * seconds, the kind of shot, a description — and prompt blocks keyed by the
 * profile, each edited on its own and copied on its own. The scaffold is
 * never typed into: the predecessor kept a scene as a markdown file edited
 * whole, and most of them were one save away from corruption. Above the
 * board sits the context every scene shares, which is a version of the
 * `context` role — a body with revisions, read by the assistant like any
 * other text — and is edited where versions are edited.
 */
export function ScenesTab({ work }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const client = useQueryClient()
  const vocabulary = vocabularyOf(profile.config, work.kind)
  const [shotType, setShotType] = useState<string | undefined>(undefined)
  // How wide the board's pane actually is. The table may be wider than it —
  // eight columns do not fit a card on a laptop — and an open row must stay
  // inside what the eye can see rather than inheriting the table's width.
  const [paneWidth, setPaneWidth] = useState<number | null>(null)
  const watching = useRef<ResizeObserver | null>(null)
  // A callback ref rather than an effect: the pane is drawn only once the
  // board has rows, and an effect that runs before that measures nothing and
  // never runs again. This fires the moment the node arrives and again when
  // it leaves.
  //
  // `clientWidth`, not the observer's `contentRect`: the pane is the box that
  // scrolls and its content is the table, which is wider. What an open row
  // must fit inside is the part a person can see.
  const pane = (box: HTMLDivElement | null) => {
    watching.current?.disconnect()
    watching.current = null
    if (box === null) return
    setPaneWidth(box.clientWidth)
    const watch = new ResizeObserver(() => {
      setPaneWidth(box.clientWidth)
    })
    watch.observe(box)
    watching.current = watch
  }

  // Which frame the viewer is showing, if it is open. Held with the other
  // board-wide state so the hooks run in the same order every render.
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

  const scenes = useQuery({
    queryKey: keys.scenesFor(work.id),
    queryFn: () => listScenes(work.id),
  })

  const refresh = () => {
    for (const key of REFRESHED) void client.invalidateQueries({ queryKey: key })
  }

  const add = useMutation({
    mutationFn: () => createScene({ work_id: work.id }),
    onSuccess: (created) => {
      refresh()
      say.ok(t('scenes.added', { number: created.position }))
    },
    onError: (cause) => say.failedTo(t('toast.sceneSaveFailed'), cause),
  })

  const patch = useMutation({
    mutationFn: ({ id, changes }: { id: string; changes: ScenePatch }) => updateScene(id, changes),
    onSuccess: () => {
      announceEdited({ client, message: t('toast.sceneEdited'), refresh: REFRESHED })
    },
    onError: (cause) => {
      // The field keeps what was typed; the board is re-read so it shows
      // what is actually stored.
      refresh()
      say.failedTo(t('toast.sceneSaveFailed'), cause)
    },
  })

  // Who and where: what each scene is about, and every note it could name.
  // Read for the whole board in one go rather than per row.
  const about = useQuery({
    queryKey: [...keys.scenes, 'notes', work.id],
    queryFn: () => listSceneNotes(work.id),
  })
  // The pictures of the whole board, in one read for the same reason.
  const frames = useQuery({
    queryKey: keys.sceneFramesFor(work.id),
    queryFn: () => listSceneFrames(work.id),
  })
  const framesForScene = framesByScene(frames.data ?? [])

  const noteKinds = profile.config.note_kinds ?? []
  const castable = useQuery({
    queryKey: [...keys.notes, 'castable'],
    queryFn: () => listNotes(),
    enabled: noteKinds.length > 0,
  })
  const cast = (castable.data ?? []).filter((note) =>
    noteKinds.some((kind) => kind.key === note.kind),
  )
  // Which note the board is narrowed to: "every scene with her in it".
  const [withNote, setWithNote] = useState<string | undefined>(undefined)

  const aboutByScene = new Map<string, SceneNote[]>()
  for (const link of about.data ?? []) {
    const held = aboutByScene.get(link.scene_id)
    if (held === undefined) aboutByScene.set(link.scene_id, [link])
    else held.push(link)
  }

  const attach = useMutation({
    mutationFn: ({ sceneId, noteId }: { sceneId: string; noteId: string }) =>
      attachSceneNote(sceneId, noteId),
    onSuccess: () => refresh(),
    onError: (cause) => say.failedTo(t('toast.sceneSaveFailed'), cause),
  })
  const detach = useMutation({
    mutationFn: ({ sceneId, noteId }: { sceneId: string; noteId: string }) =>
      detachSceneNote(sceneId, noteId),
    onSuccess: () => refresh(),
    onError: (cause) => say.failedTo(t('toast.sceneSaveFailed'), cause),
  })

  // What the board could be framed from: the first work this one is made
  // from. Nothing to frame without it, and the empty board says so.
  const links = useQuery({
    queryKey: [...keys.links, work.id],
    queryFn: () => listLinks(work.id),
  })
  const donor = links.data?.sources[0]
  const donorRoles =
    donor === undefined ? [] : vocabularyOf(profile.config, donor.source_kind).version_roles

  const frame = useMutation({
    mutationFn: (role: string) => frameScenes(work.id, role),
    onSuccess: (framed) => {
      refresh()
      say.ok(t('scenes.framed', { count: framed.length }))
    },
    onError: (cause) => say.failedTo(t('toast.sceneFrameFailed'), cause),
  })

  // The board's first timing: the work's length divided between the scenes,
  // dragged by hand from there. Refused when the work has no length, and the
  // refusal says where to give it one.
  const time = useMutation({
    mutationFn: () => timeScenes(work.id),
    onSuccess: (timed) => {
      refresh()
      say.ok(t('scenes.timed', { count: timed.length }))
    },
    onError: (cause) => say.failedTo(t('toast.sceneTimeFailed'), cause),
  })

  const remove = useMutation({
    mutationFn: (scene: Scene) => deleteScene(scene.id),
    onSuccess: (deletionId, scene) =>
      announceDeleted({
        client,
        deletionId,
        message: t('toast.sceneDeleted', { number: scene.position }),
        refresh: REFRESHED,
      }),
    onError: (cause) => say.failedTo(t('toast.sceneSaveFailed'), cause),
  })

  // A kind that names no kinds of shot and no blocks has no storyboard: the
  // tab says where to give it one rather than drawing an empty board.
  if (vocabulary.shot_types.length === 0 && vocabulary.scene_blocks.length === 0) {
    return <p className="text-sm text-dim">{t('scenes.noneForKind')}</p>
  }

  const all = scenes.data ?? []
  const counts = new Map<string, number>()
  for (const scene of all) {
    if (scene.shot_type !== null) counts.set(scene.shot_type, (counts.get(scene.shot_type) ?? 0) + 1)
  }
  const byShot = shotType === undefined ? all : all.filter((scene) => scene.shot_type === shotType)
  const shown =
    withNote === undefined
      ? byShot
      : byShot.filter((scene) =>
          (aboutByScene.get(scene.id) ?? []).some((link) => link.note_id === withNote),
        )

  // The viewer walks the scenes that have something to show, in board order.
  // Built from what is on screen, so a filtered board walks only what it shows
  // — the arrows never lead somewhere the person cannot see they are.
  const withFrames = shown.filter((scene) => (framesForScene.get(scene.id) ?? []).length > 0)
  const viewingAt = viewing
    ? withFrames.findIndex((scene) => scene.id === viewing.sceneId)
    : -1
  const canStep = (direction: -1 | 1) =>
    viewingAt >= 0 && viewingAt + direction >= 0 && viewingAt + direction < withFrames.length
  const step = (direction: -1 | 1) => {
    if (!canStep(direction)) return
    const next = withFrames[viewingAt + direction]
    if (!next) return
    const frames = framesForScene.get(next.id) ?? []
    // The chosen frame is what the scene *is*; without one, its first.
    const frame = chosenFrame(frames) ?? frames[0]
    if (frame) setViewing({ sceneId: next.id, number: next.position, frame })
  }

  // Whether the profile has anything to offer here: the empty board is an
  // entrance to the actions when there are actions, and a plain board when
  // there are none.
  const hasActions = actionsFor(profile.config.prompts, work.kind, 'work').length > 0

  return (
    <div className="flex flex-col gap-4">
      {vocabulary.version_roles.some((role) => role.key === CONTEXT_ROLE) && (
        <ContextPanel workId={work.id} />
      )}

      {/* The profile's actions for this kind, on the board's own tab: the
          plot from the source, the board from the plot. A click starts a
          task and the answer comes back as a proposal, applied with one
          button; nothing here is written by the assistant itself. */}
      <ActionBar workId={work.id} hint={t('scenes.actionsHint')} />

      {/* The kind of shot as a row of chips, the way the catalogue narrows to
          a kind: "show me every detail" is one click, and the chip that is
          on turns off. Hidden while the kind names no kinds of shot. */}
      {vocabulary.shot_types.length > 0 && (
        <div role="group" aria-label={t('scenes.shotType')} className="flex flex-wrap items-center gap-2">
          {[
            { key: undefined, label: t('scenes.allShots'), count: all.length },
            ...vocabulary.shot_types.map((shot) => ({
              key: shot.key,
              label: shot.label,
              count: counts.get(shot.key) ?? 0,
            })),
          ].map((entry) => {
            const active = shotType === entry.key
            return (
              <button
                key={entry.key ?? ''}
                type="button"
                aria-pressed={active}
                onClick={() => setShotType(active ? undefined : entry.key)}
                className={cn(
                  'cursor-pointer rounded-full border px-2.5 py-0.5 text-[11.5px] transition-colors',
                  active
                    ? 'border-transparent bg-accent-soft font-semibold text-accent-2'
                    : 'border-line text-dim hover:border-line-2 hover:text-text',
                )}
              >
                {entry.label}
                <span className="ml-1.5 text-[10.5px] text-faint tabular-nums">{entry.count}</span>
              </button>
            )
          })}
        </div>
      )}

      {/* Who and where, as a row of chips beside the kinds of shot: "every
          scene with her in it" is one click. Only the notes some scene of
          this board actually names — a cast list of everyone in the
          workspace would be a list nobody reads. */}
      {(() => {
        const named = new Map<string, SceneNote>()
        for (const link of about.data ?? []) if (!named.has(link.note_id)) named.set(link.note_id, link)
        if (named.size === 0) return null
        return (
          <div role="group" aria-label={t('scenes.about')} className="flex flex-wrap items-center gap-2">
            {[...named.values()].map((link) => {
              const active = withNote === link.note_id
              return (
                <button
                  key={link.note_id}
                  type="button"
                  aria-pressed={active}
                  onClick={() => setWithNote(active ? undefined : link.note_id)}
                  className={cn(
                    'cursor-pointer rounded-full border px-2.5 py-0.5 text-[11.5px] transition-colors',
                    active
                      ? 'border-transparent bg-accent-soft font-semibold text-accent-2'
                      : 'border-line text-dim hover:border-line-2 hover:text-text',
                  )}
                >
                  {link.note_title ?? t('scenes.untitledNote')}
                </button>
              )
            })}
          </div>
        )
      })()}

      {scenes.isPending && <Skeleton className="h-24 w-full" />}
      {scenes.isError && (
        <p role="alert" className="text-sm text-bad">
          {t('toast.loadFailed')}
        </p>
      )}

      {scenes.data !== undefined && all.length === 0 && (
        <div className="flex flex-col gap-2">
          <p className="text-sm text-dim">
            {t(hasActions ? 'scenes.emptyWithActions' : 'scenes.empty')}
          </p>
          {/* The frame from the source text: one scene per part the lyric
              marks out. Shown only when there is a source to read and roles
              to read it in — a button that can only refuse is no entrance. */}
          {donorRoles.length > 0 && (
            <div className="flex flex-wrap items-center gap-2">
              <span className="text-sm text-dim">{t('scenes.frameFrom')}</span>
              {donorRoles.map((role) => (
                <Button
                  key={role.key}
                  variant="soft"
                  size="sm"
                  disabled={frame.isPending}
                  onClick={() => frame.mutate(role.key)}
                  title={t('scenes.frameHint')}
                >
                  <Rows3 aria-hidden className="size-3.5" />
                  {role.label}
                </Button>
              ))}
            </div>
          )}
        </div>
      )}
      {scenes.data !== undefined && all.length > 0 && shown.length === 0 && (
        <p className="text-sm text-dim">{t('scenes.noneOfShot')}</p>
      )}

      {/* The board as one table: fifty scenes read as a list of rows, not as
          fifty stacked cards you scroll past to compare two timings. The box
          scrolls sideways itself and the headings stay, the way the catalogue
          does it — a table you scroll is a table whose columns must remain
          named. What does not fit a row — the description and the prompt
          blocks — opens under it. */}
      {shown.length > 0 && (
        <div ref={pane} className="min-w-0 overflow-x-auto">
          {/* `table-fixed` so the description is the column that gives way,
              and a minimum width so it does not give way to nothing: with
              every other column fixed, eight columns in a 926px pane left
              the description six pixels wide — measured, on a fifty-scene
              board. Below the minimum the pane scrolls sideways, which is
              the honest answer at that width; above it the description takes
              everything the others do not. */}
          <table className="w-full min-w-[68rem] table-fixed text-sm">
            <thead className="sticky top-0 z-10 bg-bg">
              <tr className="border-b border-line text-left text-xs uppercase tracking-wide text-dim">
                <th className="w-10 px-2 py-2 text-right">{t('scenes.number')}</th>
                <th className="w-36 px-2 py-2">{t('scenes.section')}</th>
                <th className="w-44 px-2 py-2">{t('scenes.span')}</th>
                {vocabulary.shot_types.length > 0 && (
                  <th className="w-36 px-2 py-2">{t('scenes.shotType')}</th>
                )}
                {/* No width: the column that takes what is left, and truncates. */}
                <th className="px-2 py-2">{t('scenes.description')}</th>
                {noteKinds.length > 0 && <th className="w-36 px-2 py-2">{t('scenes.about')}</th>}
                <th className="w-28 px-2 py-2">{t('scenes.readiness')}</th>
                <th className="w-24 px-2 py-2" />
              </tr>
            </thead>
            <tbody>
              {shown.map((scene) => (
                // Keyed on the moment it last changed as well as its id, so a
                // change from elsewhere — an undo, another view — redraws the
                // fields with what is stored rather than what was typed here.
                <SceneRow
                  key={`${scene.id}:${scene.updated_at}`}
                  scene={scene}
                  workId={work.id}
                  vocabulary={vocabulary}
                  open={opened.has(scene.id)}
                  onToggle={() => toggleOpen(scene.id)}
                  saving={patch.isPending && patch.variables?.id === scene.id}
                  onPatch={(changes) => patch.mutate({ id: scene.id, changes })}
                  onDelete={() => remove.mutate(scene)}
                  paneWidth={paneWidth}
                  about={aboutByScene.get(scene.id) ?? []}
                  frames={framesForScene.get(scene.id) ?? []}
                  onViewFrame={(frame) =>
                    setViewing({ sceneId: scene.id, number: scene.position, frame })
                  }
                  cast={cast}
                  onAttach={(noteId) => attach.mutate({ sceneId: scene.id, noteId })}
                  onDetach={(noteId) => detach.mutate({ sceneId: scene.id, noteId })}
                />
              ))}
            </tbody>
          </table>
        </div>
      )}

      <div className="flex flex-wrap items-center gap-2">
        <Button variant="soft" size="sm" disabled={add.isPending} onClick={() => add.mutate()}>
          <Plus aria-hidden className="size-3.5" />
          {t('scenes.add')}
        </Button>
        {all.length > 0 && (
          <Button
            variant="soft"
            size="sm"
            disabled={time.isPending}
            onClick={() => time.mutate()}
            title={t('scenes.timeHint')}
          >
            <Clock aria-hidden className="size-3.5" />
            {t('scenes.time')}
          </Button>
        )}
      </div>

      <FrameViewer
        viewing={viewing}
        onClose={() => setViewing(null)}
        onStep={step}
        canStep={canStep}
      />
    </div>
  )
}

/**
 * What every scene shares — the hero, the palette, the lens — read from the
 * current version of the `context` role. It is a body with revisions, so it
 * is edited where bodies are edited; the button opens that lane.
 */
function ContextPanel({ workId }: { workId: string }) {
  const { t } = useTranslation()
  const navigate = useNavigate()

  const versions = useQuery({
    queryKey: keys.versions(workId),
    queryFn: () => listVersions(workId),
  })
  const current = (versions.data ?? []).find(
    (version) => version.role === CONTEXT_ROLE && version.is_current,
  )
  const body = useQuery({
    queryKey: keys.version(current?.id ?? ''),
    queryFn: () => getVersion(current!.id),
    enabled: current !== undefined,
  })

  return (
    <Panel className="flex flex-col gap-2 p-4">
      <div className="flex items-center gap-2">
        <h3 className="flex-1 text-sm font-semibold">{t('scenes.context')}</h3>
        <Button
          size="sm"
          onClick={() => void navigate(`/works/${workId}/versions?role=${CONTEXT_ROLE}`)}
        >
          {current === undefined ? t('scenes.writeContext') : t('scenes.editContext')}
        </Button>
      </div>
      {current === undefined && versions.data !== undefined && (
        <p className="text-sm text-dim">{t('scenes.noContext')}</p>
      )}
      {body.data != null && body.data.body.trim() !== '' && (
        <Markdown body={body.data.body} className="text-sm" />
      )}
    </Panel>
  )
}

/**
 * One scene as a row of the board: its number, the part it plays against,
 * its seconds, the kind of shot, the first of its description, and how far
 * it is filled in. Every field saves when left; nothing here asks to be
 * saved.
 *
 * What does not fit a row opens under it in a second row: the description
 * in full, and a box per prompt block with its own copy button. A table of
 * fifty scenes is readable because the long text is folded away, not
 * because it was left out.
 */
function SceneRow({
  scene,
  workId,
  vocabulary,
  open,
  onToggle,
  saving,
  onPatch,
  onDelete,
  paneWidth,
  about,
  frames,
  onViewFrame,
  cast,
  onAttach,
  onDetach,
}: {
  scene: Scene
  workId: string
  vocabulary: Vocabulary
  open: boolean
  onToggle: () => void
  saving: boolean
  onPatch: (changes: ScenePatch) => void
  onDelete: () => void
  /** The width of the board's visible pane, when it has been measured: the
      open row is held to it rather than to the table's own width. */
  paneWidth: number | null
  /** Who is in this scene, where it happens. */
  about: SceneNote[]
  /** The pictures drawn for it, in order. */
  frames: SceneFrame[]
  onViewFrame: (frame: SceneFrame) => void
  /** Every note the profile lets a scene name. */
  cast: Note[]
  onAttach: (noteId: string) => void
  onDetach: (noteId: string) => void
}) {
  const { t } = useTranslation()
  const [starts, setStarts] = useState(formatSeconds(scene.starts_at))
  const [ends, setEnds] = useState(formatSeconds(scene.ends_at))

  const saveTime = (field: 'starts_at' | 'ends_at', text: string, stored: number | null) => {
    const parsed = parseTimecode(text)
    if (!parsed.ok) {
      say.warn(t('scenes.badTime', { text }))
      // Back to what is stored: a field left showing a value that was not
      // saved would be a second truth.
      ;(field === 'starts_at' ? setStarts : setEnds)(formatSeconds(stored))
      return
    }
    if (parsed.seconds !== stored) onPatch({ [field]: parsed.seconds })
  }

  const saveBlock = (key: string, text: string) => {
    const blocks: Record<string, string> = { ...scene.blocks }
    if (text.trim() === '') delete blocks[key]
    else blocks[key] = text
    if ((scene.blocks[key] ?? '') !== (blocks[key] ?? '')) onPatch({ blocks })
  }

  // Blocks the scene holds under keys the profile no longer names: shown,
  // not editable, so text is never hidden and never typed into blindly.
  const unlisted = Object.keys(scene.blocks).filter(
    (key) => !vocabulary.scene_blocks.some((block) => block.key === key),
  )

  const readiness = readinessOf(scene, vocabulary.scene_blocks, frames)
  const chosen = chosenFrame(frames)
  const columns = 6 + (vocabulary.shot_types.length > 0 ? 1 : 0) + (cast.length > 0 ? 1 : 0)
  const named = new Set(about.map((link) => link.note_id))

  return (
    <>
      <tr className="border-b border-line align-middle">
        <td className="px-2 py-1.5 text-right text-sm font-semibold tabular-nums">
          <span className="flex items-center justify-end gap-1.5">
            {/* The frame the scene is cut from, beside its number: the board
                answers "which scene is this picture" and "which picture is
                this scene" in the same glance — what the predecessor could
                not do. A thumbnail rather than a column, so a board without
                pictures costs no width. */}
            {chosen && (
              <button
                type="button"
                onClick={() => onViewFrame(chosen)}
                title={chosen.original_name ?? t('scenes.openFrame')}
                aria-label={t('scenes.openFrame')}
                className="overflow-hidden rounded border border-good"
              >
                <img
                  src={fileSrc(chosen.path)}
                  alt=""
                  className="size-7 object-contain"
                />
              </button>
            )}
            {scene.position}
          </span>
        </td>
        <td className="px-2 py-1.5">
          <Input
            className="w-36"
            defaultValue={scene.section ?? ''}
            placeholder={t('scenes.sectionPlaceholder')}
            aria-label={t('scenes.section')}
            onBlur={(event) => {
              const section = event.target.value.trim()
              if (section !== (scene.section ?? ''))
                onPatch({ section: section === '' ? null : section })
            }}
          />
        </td>
        <td className="px-2 py-1.5">
          <span className="flex items-center gap-1 text-xs text-dim">
            <Input
              className="w-20 text-center font-mono text-xs tabular-nums"
              value={starts}
              placeholder="0:00"
              aria-label={t('scenes.startsAt')}
              onChange={(event) => setStarts(event.target.value)}
              onBlur={(event) => saveTime('starts_at', event.target.value, scene.starts_at)}
            />
            <span aria-hidden>–</span>
            <Input
              className="w-20 text-center font-mono text-xs tabular-nums"
              value={ends}
              placeholder="0:00"
              aria-label={t('scenes.endsAt')}
              onChange={(event) => setEnds(event.target.value)}
              onBlur={(event) => saveTime('ends_at', event.target.value, scene.ends_at)}
            />
          </span>
        </td>
        {vocabulary.shot_types.length > 0 && (
          <td className="px-2 py-1.5">
            <Select
              className="w-36"
              aria-label={t('scenes.shotType')}
              value={scene.shot_type ?? ''}
              placeholder={t('scenes.noShot')}
              onChange={(value) => onPatch({ shot_type: value === '' ? null : value })}
              options={vocabulary.shot_types.map((shot) => ({
                value: shot.key,
                label: shot.label,
              }))}
            />
          </td>
        )}
        {/* The description in one line, as much of it as the column holds.
            Clicking opens the row rather than editing in place: a paragraph
            typed into a table cell is a paragraph you cannot see. */}
        <td className="px-2 py-1.5">
          <button
            type="button"
            onClick={onToggle}
            aria-expanded={open}
            className="flex w-full cursor-pointer items-center gap-1.5 text-left text-sm transition-colors hover:text-accent-2"
          >
            <ChevronRight
              aria-hidden
              className={cn(
                'size-3.5 shrink-0 text-faint transition-transform',
                open && 'rotate-90',
              )}
            />
            <span className={cn('truncate', scene.description === '' && 'text-faint')}>
              {scene.description === '' ? t('scenes.describeIt') : scene.description}
            </span>
          </button>
        </td>
        {cast.length > 0 && (
          // Who and where, as names rather than as text inside the shot: a
          // hero written three ways across fifty scenes is three heroes to
          // anything that reads them. Chosen in the open row below.
          <td className="px-2 py-1.5">
            <span className="flex flex-wrap gap-1">
              {about.length === 0 ? (
                <span className="text-xs text-faint">—</span>
              ) : (
                about.map((link) => (
                  <span
                    key={link.note_id}
                    className="rounded-full border border-line px-1.5 py-0.5 text-[11px] text-dim"
                  >
                    {link.note_title ?? t('scenes.untitledNote')}
                  </span>
                ))
              )}
            </span>
          </td>
        )}
        <td className="px-2 py-1.5">
          <ReadinessMark readiness={readiness} blocks={vocabulary.scene_blocks.length} />
        </td>
        <td className="px-2 py-1.5">
          <span className="flex items-center justify-end gap-1">
            {saving && <span className="text-xs text-faint">{t('save.saving')}</span>}
            {/* The profile's scene actions — the prompts for this scene —
                start on this row and come back as a revision of it. */}
            <ActionBar workId={workId} sceneId={scene.id} compact />
            <Button
              variant="danger"
              size="icon-sm"
              title={t('scenes.delete')}
              aria-label={t('scenes.delete')}
              onClick={onDelete}
            >
              <Trash2 aria-hidden className="size-3.5" />
            </Button>
          </span>
        </td>
      </tr>

      {open && (
        <tr className="border-b border-line bg-soft/40">
          <td colSpan={columns} className="px-2 py-3">
            {/* The open row is read, not scrolled. The table is wider than
                the pane whenever the board carries every column, and a row
                that inherited that width put the last prompt block off the
                edge — measured. `sticky left-0` with the pane's own width
                holds the panel where the eye is while the table scrolls
                under it. */}
            <div
              className="sticky left-0 flex flex-col gap-3"
              style={paneWidth === null ? undefined : { width: paneWidth - 16 }}
            >
              <Textarea
                autoResize
                maxRows={8}
                rows={2}
                defaultValue={scene.description}
                placeholder={t('scenes.descriptionPlaceholder')}
                aria-label={t('scenes.description')}
                onBlur={(event) => {
                  if (event.target.value !== scene.description)
                    onPatch({ description: event.target.value })
                }}
              />

              {cast.length > 0 && (
                <div className="flex flex-wrap items-center gap-1.5">
                  <span className="text-xs text-dim">{t('scenes.aboutHint')}</span>
                  {cast.map((note) => {
                    const chosen = named.has(note.id)
                    return (
                      <button
                        key={note.id}
                        type="button"
                        aria-pressed={chosen}
                        onClick={() => (chosen ? onDetach(note.id) : onAttach(note.id))}
                        className={cn(
                          'cursor-pointer rounded-full border px-2 py-0.5 text-[11.5px] transition-colors',
                          chosen
                            ? 'border-transparent bg-accent-soft font-semibold text-accent-2'
                            : 'border-line text-dim hover:border-line-2 hover:text-text',
                        )}
                      >
                        {note.title ?? t('scenes.untitledNote')}
                      </button>
                    )
                  })}
                </div>
              )}

              {vocabulary.scene_blocks.length > 0 && (
                <div className="grid grid-cols-[repeat(auto-fit,minmax(220px,1fr))] gap-3">
                  {vocabulary.scene_blocks.map((block) => (
                    <BlockBox
                      key={block.key}
                      block={block}
                      text={scene.blocks[block.key] ?? ''}
                      workId={workId}
                      sceneId={scene.id}
                      onSave={(text) => saveBlock(block.key, text)}
                    />
                  ))}
                </div>
              )}

              {unlisted.length > 0 && (
                <div className="grid grid-cols-[repeat(auto-fit,minmax(220px,1fr))] gap-3">
                  {unlisted.map((key) => (
                    <BlockBox
                      key={key}
                      block={{ key, label: key, hint: t('scenes.unlistedBlock') }}
                      text={scene.blocks[key] ?? ''}
                    />
                  ))}
                </div>
              )}

              {/* The pictures, under the prompts they came from: the prompt is
                  copied out to a generator and the answer comes back here. */}
              <SceneFrames
                workId={workId}
                sceneId={scene.id}
                number={scene.position}
                frames={frames}
                onOpen={onViewFrame}
              />
            </div>
          </td>
        </tr>
      )}
    </>
  )
}

/**
 * How far a scene is filled in, as a word and a dot.
 *
 * Computed from what the scene holds, never stored: ADR 0020 turned down a
 * status column for the frame because it derives from what is there. A kind
 * with no prompt blocks asks only for a description, and the mark's tooltip
 * says so rather than implying blocks nobody named.
 */
function ReadinessMark({ readiness, blocks }: { readiness: Readiness; blocks: number }) {
  const { t } = useTranslation()
  // `shot` is a step past `ready`, so it reads as the same good colour filled
  // in rather than as a different judgement.
  const tone =
    readiness === 'shot'
      ? 'bg-good ring-2 ring-good/30'
      : readiness === 'ready'
        ? 'bg-good'
        : readiness === 'started'
          ? 'bg-warn'
          : 'bg-line-2'
  const hint =
    readiness === 'shot'
      ? t('scenes.readinessHintShot')
      : blocks > 0
        ? t('scenes.readinessHint')
        : t('scenes.readinessHintNoBlocks')

  return (
    <span className="flex items-center gap-1.5 text-xs text-dim" title={hint}>
      <span aria-hidden className={cn('size-2 shrink-0 rounded-full', tone)} />
      {t(`scenes.readiness_${readiness}`)}
    </span>
  )
}

/**
 * One prompt block: a caption, a copy button, the box. Copying is the whole
 * point of a block — it goes into a generator as it is — so the button is
 * on every block, and the tick comes only after the clipboard confirms.
 */
function BlockBox({
  block,
  text,
  workId,
  sceneId,
  onSave,
}: {
  block: SceneBlock
  text: string
  /** The work and scene this block belongs to — the scene actions are
      offered here aimed at this block alone. Absent for a block the profile
      no longer names: there is nothing to regenerate into. */
  workId?: string
  sceneId?: string
  /** Absent for a block the profile no longer names: read, not written. */
  onSave?: (text: string) => void
}) {
  const { t } = useTranslation()

  const copy = () => {
    navigator.clipboard.writeText(text).then(
      () => say.ok(t('scenes.copied', { block: block.label })),
      (cause: unknown) => say.failedTo(t('scenes.copied', { block: block.label }), cause),
    )
  }

  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center gap-1">
        <span className="flex-1 text-2xs font-semibold uppercase tracking-caption text-faint">
          {block.label}
        </span>
        <Button
          variant="icon"
          size="icon-sm"
          title={t('scenes.copy', { block: block.label })}
          aria-label={t('scenes.copy', { block: block.label })}
          disabled={text === ''}
          onClick={copy}
        >
          <Copy aria-hidden className="size-3.5" />
        </Button>
        {/* The profile's scene actions, aimed at this block: the animation
            rewritten without touching the still. The answer is held to this
            block, and what it brings is laid over the scene rather than put
            in its place. */}
        {workId !== undefined && sceneId !== undefined && (
          <ActionBar workId={workId} sceneId={sceneId} block={block.key} compact />
        )}
      </div>
      <Textarea
        autoResize
        maxRows={12}
        rows={3}
        defaultValue={text}
        readOnly={onSave === undefined}
        aria-label={block.label}
        onBlur={(event) => onSave?.(event.target.value)}
      />
      {block.hint !== undefined && block.hint !== null && (
        <span className="text-xs text-faint">{block.hint}</span>
      )}
    </div>
  )
}
