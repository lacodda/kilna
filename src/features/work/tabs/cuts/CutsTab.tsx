import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from 'react-router'
import { save } from '@tauri-apps/plugin-dialog'
import { Film, Plus, Scissors, Trash2 } from 'lucide-react'
import { createCut, deleteCut, reorderCuts, updateCut } from '@/lib/api/cuts'
import { writeTextFile } from '@/lib/api/data'
import type { Cut, Work } from '@/lib/api/types'
import { bandsOf, blockerOf, lengthOf, orderMoving, totalLength, tracksOf } from '@/lib/cuts'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { formatSeconds } from '@/lib/timecode'
import { say } from '@/lib/toast'
import { announceDeleted } from '@/lib/trash'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { InlineField, timecodeCodec } from '@/components/ui/inline-field'
import { Panel, SectionLabel } from '@/components/ui/panel'
import { QueryState } from '@/components/ui/query-state'
import { Skeleton } from '@/components/ui/skeleton'
import { Frame, Scroll } from '@/components/frame'

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

  const cuts = useQuery(queries.cuts(work.id))
  const links = useQuery(queries.links(work.id))

  const add = useAppMutation({
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
    refresh: refresh.cut,
  })

  const edit = useAppMutation({
    mutationFn: ({ id, starts_at, ends_at }: { id: string; starts_at: number; ends_at: number }) =>
      updateCut(id, { starts_at, ends_at }),
    refresh: refresh.cut,
  })

  const rename = useAppMutation({
    mutationFn: ({ id, label }: { id: string; label: string | null }) => updateCut(id, { label }),
    refresh: refresh.cut,
  })

  const remove = useAppMutation({
    mutationFn: (id: string) => deleteCut(id),
    onSuccess: (deletionId) => {
      announceDeleted({
        client,
        deletionId,
        message: t('cuts.removed'),
        refresh: refresh.cut,
      })
    },
  })

  const reorder = useAppMutation({
    mutationFn: (ids: string[]) => reorderCuts(work.id, ids),
    refresh: refresh.cut,
  })

  // Two reads, and the tab needs both: the splice and the works it can take
  // from. A failure of either offers to read both again.
  if (cuts.isPending || links.isPending || cuts.isError || links.isError) {
    return (
      <Frame>
        <QueryState
          pending={cuts.isPending || links.isPending}
          error={cuts.isError || links.isError ? t('toast.loadFailed') : null}
          skeleton={<Skeleton className="h-40 w-full" />}
          onRetry={() => {
            void cuts.refetch()
            void links.refetch()
          }}
          retryLabel={t('crash.retry')}
        >
          {null}
        </QueryState>
      </Frame>
    )
  }

  const splice = cuts.data ?? []
  const donors = links.data?.sources ?? []
  const tracks = tracksOf(splice)

  return (
    <Frame>
      <Scroll label={t('card.tab.cuts')} contentClassName="flex flex-col gap-4">
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
                  <Plus aria-hidden />
                  {t('cuts.takeFrom', { title: link.source_title })}
                </Button>
              ))}
            </div>
          )}
        </Panel>

        <ShotList workId={work.id} title={work.title} />
      </Scroll>
    </Frame>
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
        <Button variant="link" onClick={onOpen}>
          <Film aria-hidden />
          {track.source_title}
        </Button>
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
              style={{
                left: `${band.left * 100}%`,
                width: `${Math.max(band.width, 0.004) * 100}%`,
              }}
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
              label={t('scenes.startsAt')}
              value={cut.starts_at}
              onCommit={(seconds) => onEdit(cut.id, seconds, cut.ends_at)}
              disabled={busy}
            />
            <span className="text-faint">–</span>
            <Span
              label={t('scenes.endsAt')}
              value={cut.ends_at}
              onCommit={(seconds) => onEdit(cut.id, cut.starts_at, seconds)}
              disabled={busy}
            />
            <span className="w-12 shrink-0 text-xs text-faint">{formatSeconds(lengthOf(cut))}</span>
            {/* The name is stored, so it is shown and typeable: a column the
                schema keeps and no screen draws is how a field dies. */}
            <Label
              value={cut.label}
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
 * The field holds the text while it is being typed and commits on leaving or
 * Enter (InlineField) — committing per keystroke would send `4` on the way to
 * `48` and the backend would take it, moving the other end under the cursor.
 */
function Span({
  label,
  value,
  onCommit,
  disabled,
}: {
  label: string
  value: number
  onCommit: (seconds: number) => void
  disabled: boolean
}) {
  return (
    <InlineField
      className="w-20 shrink-0"
      label={label}
      labelHidden
      codec={timecodeCodec}
      value={value}
      onCommit={(seconds) => {
        // A timecode nobody can read is refused by the field itself; an empty
        // one is not an end either. Either way the field goes back to what is
        // stored, which is the one value that is certainly true.
        if (seconds === null || seconds === value) return false
        onCommit(seconds)
      }}
      disabled={disabled}
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
  value: string | null
  onCommit: (label: string | null) => void
  disabled: boolean
}) {
  const { t } = useTranslation()

  return (
    <InlineField
      className="flex-1"
      label={t('cuts.name')}
      labelHidden
      placeholder={t('cuts.namePlaceholder')}
      value={value}
      onCommit={(text) => {
        const trimmed = (text ?? '').trim()
        // Blank is no name, not an empty one: the two read alike and telling
        // them apart would only let one of them hide. Only spaces around the
        // same name is no change, and the box goes back to it.
        if (trimmed === (value ?? '')) return false
        onCommit(trimmed === '' ? null : trimmed)
      }}
      disabled={disabled}
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
  const shots = useQuery(queries.shots(workId))
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
        <Scissors aria-hidden />
        {t('cuts.save')}
      </Button>
    </Panel>
  )
}
