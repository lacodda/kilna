import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from 'react-router'
import { Clock, Copy, Plus, Trash2 } from 'lucide-react'
import {
  createScene,
  deleteScene,
  getVersion,
  listScenes,
  listVersions,
  timeScenes,
  updateScene,
  type Scene,
  type SceneBlock,
  type ScenePatch,
  type Work,
} from '@/lib/api'
import { announceEdited } from '@/lib/edited'
import { keys } from '@/lib/query'
import { formatSeconds, parseTimecode } from '@/lib/timecode'
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

interface Props {
  work: Work
}

/** The role that holds what every scene has in common: hero, palette, lens. */
const CONTEXT_ROLE = 'context'

/** The queries a scene changes: the board, and the feed. */
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
  const shown = shotType === undefined ? all : all.filter((scene) => scene.shot_type === shotType)

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

      {scenes.isPending && <Skeleton className="h-24 w-full" />}
      {scenes.isError && (
        <p role="alert" className="text-sm text-bad">
          {t('toast.loadFailed')}
        </p>
      )}

      {scenes.data !== undefined && all.length === 0 && (
        <p className="text-sm text-dim">{t(hasActions ? 'scenes.emptyWithActions' : 'scenes.empty')}</p>
      )}
      {scenes.data !== undefined && all.length > 0 && shown.length === 0 && (
        <p className="text-sm text-dim">{t('scenes.noneOfShot')}</p>
      )}

      {shown.length > 0 && (
        <ol className="flex flex-col gap-3">
          {shown.map((scene) => (
            // Keyed on the moment it last changed as well as its id, so a
            // change from elsewhere — an undo, another view — redraws the
            // fields with what is stored rather than what was typed here.
            <SceneCard
              key={`${scene.id}:${scene.updated_at}`}
              scene={scene}
              workId={work.id}
              vocabulary={vocabulary}
              saving={patch.isPending && patch.variables?.id === scene.id}
              onPatch={(changes) => patch.mutate({ id: scene.id, changes })}
              onDelete={() => remove.mutate(scene)}
            />
          ))}
        </ol>
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
 * One scene: its number, the part it plays against, its seconds, the kind
 * of shot, a description, and a box per prompt block with its own copy
 * button. Every field saves when left; nothing here asks to be saved.
 */
function SceneCard({
  scene,
  workId,
  vocabulary,
  saving,
  onPatch,
  onDelete,
}: {
  scene: Scene
  workId: string
  vocabulary: Vocabulary
  saving: boolean
  onPatch: (changes: ScenePatch) => void
  onDelete: () => void
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

  return (
    <li>
      <Panel className="flex flex-col gap-3 p-4">
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-sm font-semibold tabular-nums">#{scene.position}</span>
          <Input
            className="w-36"
            defaultValue={scene.section ?? ''}
            placeholder={t('scenes.sectionPlaceholder')}
            aria-label={t('scenes.section')}
            onBlur={(event) => {
              const section = event.target.value.trim()
              if (section !== (scene.section ?? '')) onPatch({ section: section === '' ? null : section })
            }}
          />
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
          {vocabulary.shot_types.length > 0 && (
            <Select
              className="w-36"
              aria-label={t('scenes.shotType')}
              value={scene.shot_type ?? ''}
              placeholder={t('scenes.noShot')}
              onChange={(value) => onPatch({ shot_type: value === '' ? null : value })}
              options={vocabulary.shot_types.map((shot) => ({ value: shot.key, label: shot.label }))}
            />
          )}
          <span className="flex-1" />
          {saving && <span className="text-xs text-faint">{t('save.saving')}</span>}
          {/* The profile's scene actions — the prompts for this scene — start
              on this row and come back as a revision of it. */}
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
        </div>

        <Textarea
          autoResize
          maxRows={8}
          rows={2}
          defaultValue={scene.description}
          placeholder={t('scenes.descriptionPlaceholder')}
          aria-label={t('scenes.description')}
          onBlur={(event) => {
            if (event.target.value !== scene.description) onPatch({ description: event.target.value })
          }}
        />

        {vocabulary.scene_blocks.length > 0 && (
          <div className="grid grid-cols-[repeat(auto-fit,minmax(220px,1fr))] gap-3">
            {vocabulary.scene_blocks.map((block) => (
              <BlockBox
                key={block.key}
                block={block}
                text={scene.blocks[block.key] ?? ''}
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
      </Panel>
    </li>
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
  onSave,
}: {
  block: SceneBlock
  text: string
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
