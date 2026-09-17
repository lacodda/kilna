import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from 'react-router'
import { save } from '@tauri-apps/plugin-dialog'
import { Film, Plus, Scissors, Trash2 } from 'lucide-react'
import {
  createCut,
  cutShotList,
  deleteCut,
  listCuts,
  listLinks,
  reorderCuts,
  updateCut,
  writeTextFile,
  type Cut,
  type Work,
} from '@/lib/api'
import { bandsOf, blockerOf, lengthOf, orderMoving, totalLength, tracksOf } from '@/lib/cuts'
import { keys } from '@/lib/query'
import { formatSeconds, parseTimecode } from '@/lib/timecode'
import { say } from '@/lib/toast'
import { announceDeleted } from '@/lib/trash'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Panel, SectionLabel } from '@/components/ui/panel'
import { Skeleton } from '@/components/ui/Skeleton'

interface Props {
  work: Work
}

/**
 * The stretches of a longer work this one is spliced from.
 *
 * The heart of it is a track per donor: the video drawn as a bar of its own
 * length, with each stretch sitting where it actually falls. That is the
 * point of a track rather than two number fields — a person deciding whether
 * to take another twelve seconds is asking "where, relative to what I already
 * took", and two numbers cannot be looked at that way.
 *
 * The seconds are still typeable underneath, because the eye places a cut and
 * the keyboard finishes it: nobody drags to exactly 48.0.
 *
 * Nothing here opens a video file. The core keeps the boundaries and hands
 * them out (decision of 2026-09-11); the cutting plugin of v1.10 does the
 * cutting, and until it exists the list can be saved out for whatever does.
 */
export function CutsTab({ work }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const navigate = useNavigate()

  const cuts = useQuery({ queryKey: keys.cutsFor(work.id), queryFn: () => listCuts(work.id) })
  const links = useQuery({ queryKey: keys.linksFor(work.id), queryFn: () => listLinks(work.id) })

  const refresh = () => {
    void client.invalidateQueries({ queryKey: keys.cuts })
    void client.invalidateQueries({ queryKey: keys.journal })
  }

  const add = useMutation({
    mutationFn: (sourceId: string) => {
      // A new stretch starts where the last one of that donor ended, so
      // taking three in a row is three clicks rather than three sums. The
      // first one starts at the beginning.
      const taken = (cuts.data ?? []).filter((cut) => cut.source_id === sourceId)
      const from = taken.reduce((latest, cut) => Math.max(latest, cut.ends_at), 0)
      return createCut({
        work_id: work.id,
        source_id: sourceId,
        starts_at: from,
        ends_at: from + DEFAULT_LENGTH,
      })
    },
    onSuccess: refresh,
    onError: (cause) => say.failed(cause),
  })

  const edit = useMutation({
    mutationFn: ({ id, starts_at, ends_at }: { id: string; starts_at: number; ends_at: number }) =>
      updateCut(id, { starts_at, ends_at }),
    onSuccess: refresh,
    onError: (cause) => say.failed(cause),
  })

  const rename = useMutation({
    mutationFn: ({ id, label }: { id: string; label: string | null }) => updateCut(id, { label }),
    onSuccess: refresh,
    onError: (cause) => say.failed(cause),
  })

  const remove = useMutation({
    mutationFn: (id: string) => deleteCut(id),
    onSuccess: (deletionId) => {
      announceDeleted({
        client,
        deletionId,
        message: t('cuts.removed'),
        refresh: [keys.cuts, keys.journal],
      })
    },
    onError: (cause) => say.failed(cause),
  })

  const reorder = useMutation({
    mutationFn: (ids: string[]) => reorderCuts(work.id, ids),
    onSuccess: refresh,
    onError: (cause) => say.failed(cause),
  })

  if (cuts.isPending || links.isPending) return <Skeleton className="h-40 w-full" />
  if (cuts.isError || links.isError) {
    return (
      <p role="alert" className="text-sm text-bad">
        {t('toast.loadFailed')}
      </p>
    )
  }

  const splice = cuts.data ?? []
  const donors = links.data?.sources ?? []
  const tracks = tracksOf(splice)

  return (
    <div className="flex flex-col gap-4">
      <Panel className="flex flex-col gap-4 p-4">
        <header className="flex items-baseline justify-between gap-3">
          <h3 className="text-sm font-semibold">{t('cuts.title')}</h3>
          {splice.length > 0 && (
            <p className="text-sm text-dim">
              {t('cuts.runs', {
                length: formatSeconds(totalLength(splice)),
                count: splice.length,
              })}
            </p>
          )}
        </header>

        {donors.length === 0 && splice.length === 0 && (
          <p className="text-sm text-dim">{t('cuts.noDonor')}</p>
        )}

        {tracks.map((track) => (
          <Track
            key={track.source_id}
            track={track}
            onOpen={() => void navigate(`/works/${track.source_id}/overview`)}
            onEdit={(id, starts_at, ends_at) => edit.mutate({ id, starts_at, ends_at })}
            onRename={(id, label) => rename.mutate({ id, label })}
            onRemove={(id) => remove.mutate(id)}
            onReorder={(ids) => reorder.mutate(ids)}
            splice={splice}
            busy={edit.isPending || rename.isPending || remove.isPending || reorder.isPending}
          />
        ))}

        {donors.length > 0 && (
          <div className="flex flex-wrap gap-2">
            {donors.map((link) => (
              <Button
                key={link.source_id}
                variant="soft"
                disabled={add.isPending}
                onClick={() => add.mutate(link.source_id)}
              >
                <Plus aria-hidden className="size-4" />
                {t('cuts.takeFrom', { title: link.source_title })}
              </Button>
            ))}
          </div>
        )}
      </Panel>

      <ShotList workId={work.id} title={work.title} />
    </div>
  )
}

/** How long a stretch is when it is first taken: long enough to see on the
    track, short enough that nobody meant it. */
const DEFAULT_LENGTH = 10

/** One donor, drawn as its own length with the stretches taken out of it. */
function Track({
  track,
  splice,
  onOpen,
  onEdit,
  onRename,
  onRemove,
  onReorder,
  busy,
}: {
  track: ReturnType<typeof tracksOf>[number]
  splice: Cut[]
  onOpen: () => void
  onEdit: (id: string, starts_at: number, ends_at: number) => void
  onRename: (id: string, label: string | null) => void
  onRemove: (id: string) => void
  onReorder: (ids: string[]) => void
  busy: boolean
}) {
  const { t } = useTranslation()
  const bands = bandsOf(track.cuts, track.source_duration)
  const [dragging, setDragging] = useState<string | null>(null)

  return (
    <section className="flex flex-col gap-2">
      <SectionLabel>
        <button type="button" className="hover:text-text" onClick={onOpen}>
          <Film aria-hidden className="mr-1 inline size-3" />
          {track.source_title}
        </button>
        {track.source_duration !== null && <span>{formatSeconds(track.source_duration)}</span>}
      </SectionLabel>

      {bands === null ? (
        // A track with no scale would place every cut at an arbitrary point
        // and look exactly as authoritative as a real one.
        <p className="text-sm text-warn">{t('cuts.noLength', { title: track.source_title })}</p>
      ) : (
        <div className="relative h-8 w-full overflow-hidden rounded-inner bg-soft">
          {bands.map((band) => (
            <div
              key={band.cut.id}
              title={`${formatSeconds(band.cut.starts_at)}–${formatSeconds(band.cut.ends_at)}`}
              className="absolute inset-y-0 rounded-inner bg-accent/70"
              style={{ left: `${band.left * 100}%`, width: `${Math.max(band.width, 0.004) * 100}%` }}
            />
          ))}
        </div>
      )}

      <ul className="flex flex-col gap-1">
        {track.cuts.map((cut) => (
          <li
            key={cut.id}
            draggable={!busy}
            onDragStart={() => setDragging(cut.id)}
            onDragEnd={() => setDragging(null)}
            onDragOver={(event) => event.preventDefault()}
            onDrop={() => {
              if (dragging !== null && dragging !== cut.id) {
                onReorder(orderMoving(splice, dragging, cut.id))
              }
              setDragging(null)
            }}
            className={cn(
              'flex items-center gap-2 rounded-inner px-2 py-1 text-sm',
              dragging === cut.id && 'opacity-50',
            )}
          >
            <span className="w-6 shrink-0 text-right text-xs text-faint">{cut.position}</span>
            <Span
              value={cut.starts_at}
              onCommit={(seconds) => onEdit(cut.id, seconds, cut.ends_at)}
              disabled={busy}
            />
            <span className="text-faint">–</span>
            <Span
              value={cut.ends_at}
              onCommit={(seconds) => onEdit(cut.id, cut.starts_at, seconds)}
              disabled={busy}
            />
            <span className="w-12 shrink-0 text-xs text-faint">
              {formatSeconds(lengthOf(cut))}
            </span>
            {/* The name is stored, so it is shown and typeable: a column the
                schema keeps and no screen draws is how a field dies. */}
            <Label
              value={cut.label ?? ''}
              onCommit={(label) => onRename(cut.id, label)}
              disabled={busy}
            />
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label={t('cuts.remove')}
              disabled={busy}
              onClick={() => onRemove(cut.id)}
            >
              <Trash2 aria-hidden className="size-4" />
            </Button>
          </li>
        ))}
      </ul>
    </section>
  )
}

/**
 * One end of a stretch, typed.
 *
 * The field holds the text while it is being typed and commits on blur or
 * Enter — committing per keystroke would send `4` on the way to `48` and the
 * backend would take it, moving the other end under the cursor.
 */
function Span({
  value,
  onCommit,
  disabled,
}: {
  value: number
  onCommit: (seconds: number) => void
  disabled: boolean
}) {
  const [text, setText] = useState<string | null>(null)
  const shown = text ?? formatSeconds(value)

  const commit = () => {
    if (text === null) return
    const parsed = parseTimecode(text)
    // A timecode nobody can read is not guessed at: the field goes back to
    // what is stored, which is the one value that is certainly true.
    if (parsed.ok && parsed.seconds !== null && parsed.seconds !== value) {
      onCommit(parsed.seconds)
    }
    setText(null)
  }

  return (
    <Input
      value={shown}
      disabled={disabled}
      onChange={(event) => setText(event.target.value)}
      onBlur={commit}
      onKeyDown={(event) => {
        if (event.key === 'Enter') event.currentTarget.blur()
        if (event.key === 'Escape') setText(null)
      }}
      className="h-7 w-20 text-center font-mono text-xs"
    />
  )
}

/**
 * What a person calls a stretch — "the hook", "the last line".
 *
 * Usually empty, and deliberately unobtrusive when it is: the picture on the
 * track says more than a name does, and a row of placeholder text would say
 * every stretch is missing something.
 */
function Label({
  value,
  onCommit,
  disabled,
}: {
  value: string
  onCommit: (label: string | null) => void
  disabled: boolean
}) {
  const { t } = useTranslation()
  const [text, setText] = useState<string | null>(null)
  const shown = text ?? value

  return (
    <Input
      value={shown}
      disabled={disabled}
      placeholder={t('cuts.namePlaceholder')}
      aria-label={t('cuts.name')}
      onChange={(event) => setText(event.target.value)}
      onBlur={() => {
        const trimmed = (text ?? '').trim()
        // Blank is no name, not an empty one: the two read alike and telling
        // them apart would only let one of them hide.
        if (text !== null && trimmed !== value) onCommit(trimmed === '' ? null : trimmed)
        setText(null)
      }}
      onKeyDown={(event) => {
        if (event.key === 'Enter') event.currentTarget.blur()
        if (event.key === 'Escape') setText(null)
      }}
      className="h-7 min-w-0 flex-1 border-transparent bg-transparent text-xs hover:border-line focus:border-line"
    />
  )
}

/**
 * What a cutter is told to do, and what is still missing from it.
 *
 * The core's whole part in making the file. Until the plugin of v1.10 exists
 * the list is saved out as JSON, which is a file any cutter can be pointed
 * at — and which is the same thing the plugin will read.
 */
function ShotList({ workId, title }: { workId: string; title: string }) {
  const { t } = useTranslation()
  const shots = useQuery({ queryKey: keys.shotsFor(workId), queryFn: () => cutShotList(workId) })
  const saving = useRef(false)

  if (shots.data === undefined) return null
  const blocker = blockerOf(shots.data)

  return (
    <Panel className="flex items-center justify-between gap-3 p-4">
      <div>
        <h3 className="text-sm font-semibold">{t('cuts.shotList')}</h3>
        <p className="mt-1 text-sm text-dim">
          {blocker === null ? t('cuts.readyToCut') : t(`cuts.blocked.${blocker}`)}
        </p>
      </div>
      <Button
        variant="soft"
        disabled={blocker !== null}
        onClick={async () => {
          if (saving.current) return
          saving.current = true
          try {
            const path = await save({
              defaultPath: `${title.replace(/[\\/:*?"<>|]/g, '-')} cuts.json`,
              filters: [{ name: 'JSON', extensions: ['json'] }],
            })
            if (typeof path === 'string') {
              await writeTextFile(path, JSON.stringify(shots.data, null, 2))
              say.ok(t('cuts.saved'))
            }
          } catch (cause) {
            say.failedTo(t('cuts.save'), cause)
          } finally {
            saving.current = false
          }
        }}
      >
        <Scissors aria-hidden className="size-4" />
        {t('cuts.save')}
      </Button>
    </Panel>
  )
}
