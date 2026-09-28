import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from 'react-router'
import { save } from '@tauri-apps/plugin-dialog'
import { Film, Plus, Scissors, X } from 'lucide-react'
import { createCut, deleteCut, reorderCuts, updateCut } from '@/lib/api/cuts'
import { writeTextFile } from '@/lib/api/data'
import type { Cut, Work } from '@/lib/api/types'
import {
  blockerOf,
  lengthOf,
  orderWithin,
  scaleOf,
  totalLength,
  tracksOf,
  type Stretch,
} from '@/lib/cuts'
import { keys } from '@/lib/query/keys'
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
import { ReorderGrip, ReorderIndicator, useReorder } from '@/components/ui/reorderable-list'
import { Skeleton } from '@/components/ui/skeleton'
import { Frame, Scroll } from '@/components/frame'
import { CutFromThis } from '@/features/work/tabs/cuts/CutFromThis'
import { CutTrack } from '@/features/work/tabs/cuts/CutTrack'

interface Props {
  work: Work
}

/**
 * The stretches of a longer work this one is spliced from - and, on a work
 * others were cut from, what they took.
 *
 * The heart of it is a track per donor: the video drawn as a bar of its own
 * length, with each stretch sitting where it actually falls. That is the
 * point of a track rather than two number fields — a person deciding whether
 * to take another twelve seconds is asking "where, relative to what I already
 * took", and two numbers cannot be looked at that way.
 *
 * A stretch is dragged along the track to move it, or by an edge to move one
 * end; the seconds are still typeable underneath, because the eye places a
 * cut and the keyboard finishes it: nobody drags to exactly 48.0.
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
  const taken = useQuery(queries.cutsFrom(work.id))

  const add = useAppMutation({
    mutationFn: (sourceId: string) => {
      // A new stretch starts where the last one of that donor ended, so
      // taking three in a row is three clicks rather than three sums. The
      // first one starts at the beginning.
      const earlier = (cuts.data ?? []).filter((cut) => cut.source_id === sourceId)
      const from = earlier.reduce((latest, cut) => Math.max(latest, cut.ends_at), 0)
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
    mutationFn: ({ id, starts_at, ends_at }: { id: string } & Stretch) =>
      updateCut(id, { starts_at, ends_at }),
    refresh: refresh.cut,
    // A dragged stretch is drawn where it was let go before the write lands
    // (see `place`); a write refused leaves it there, drawn wrong, until the
    // splice is read again.
    onError: (cause) => {
      say.failed(cause)
      void client.invalidateQueries({ queryKey: keys.cuts })
    },
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

  /** A stretch let go on the track: drawn there at once, then written. The
      splice is re-read after the write anyway; without this the stretch would
      jump back to where it was taken from and forward again when the read
      lands. */
  const place = (id: string, stretch: Stretch) => {
    client.setQueryData<Cut[]>(keys.cutsFor(work.id), (old) =>
      old?.map((cut) => (cut.id === id ? { ...cut, ...stretch } : cut)),
    )
    edit.mutate({ id, ...stretch })
  }

  // Three reads, and the tab needs all of them: the splice, the works it can
  // take from, and what was taken out of this one. A failure of any offers
  // to read them again.
  const reads = [cuts, links, taken]
  if (reads.some((read) => read.isPending || read.isError)) {
    return (
      <Frame>
        <QueryState
          pending={reads.some((read) => read.isPending)}
          error={reads.some((read) => read.isError) ? t('toast.loadFailed') : null}
          skeleton={<Skeleton className="h-40 w-full" />}
          onRetry={() => {
            for (const read of reads) void read.refetch()
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
  const takenFromThis = taken.data ?? []
  // A work only ever cut into others has no splice of its own to show, and
  // telling it to name a donor would be telling it to become something else.
  // A work with neither says how a splice begins.
  const spliced = splice.length > 0 || donors.length > 0 || takenFromThis.length === 0
  const busy = edit.isPending || rename.isPending || remove.isPending || reorder.isPending

  return (
    <Frame>
      <Scroll label={t('card.tab.cuts')} contentClassName="flex flex-col gap-2.5">
        {spliced && (
          <Panel className="flex flex-col gap-2.5 px-3 py-2.5">
            <header className="flex items-center gap-2">
              <SectionLabel>{t('cuts.title')}</SectionLabel>
              {splice.length > 0 && (
                <span className="ml-auto font-mono text-xs text-faint">
                  {t('cuts.runs', {
                    length: formatSeconds(totalLength(splice)),
                    count: splice.length,
                  })}
                </span>
              )}
            </header>

            {donors.length === 0 && splice.length === 0 && (
              <p className="text-sm text-dim">{t('cuts.noDonor')}</p>
            )}

            {tracks.map((track) => (
              <DonorTrack
                key={track.source_id}
                track={track}
                splice={splice}
                onOpen={() => void navigate(`/works/${track.source_id}/overview`)}
                onPlace={place}
                onEdit={(id, starts_at, ends_at) => edit.mutate({ id, starts_at, ends_at })}
                onRename={(id, label) => rename.mutate({ id, label })}
                onRemove={(id) => remove.mutate(id)}
                onReorder={(ids) => reorder.mutate(ids)}
                busy={busy}
              />
            ))}

            {donors.length > 0 && (
              <div className="flex flex-wrap gap-2">
                {donors.map((link) => (
                  <Button
                    key={link.source_id}
                    size="sm"
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
        )}

        {takenFromThis.length > 0 && (
          <CutFromThis work={work} cuts={takenFromThis} derived={links.data?.derived ?? []} />
        )}

        {spliced && <ShotList workId={work.id} title={work.title} />}
      </Scroll>
    </Frame>
  )
}

/** How long a stretch is when it is first taken: long enough to see on the
    track, short enough that nobody meant it. */
const DEFAULT_LENGTH = 10

/** One donor, drawn as its own length with the stretches taken out of it,
    and the stretches listed under it in the order the short plays them. */
function DonorTrack({
  track,
  splice,
  onOpen,
  onPlace,
  onEdit,
  onRename,
  onRemove,
  onReorder,
  busy,
}: {
  track: ReturnType<typeof tracksOf>[number]
  splice: Cut[]
  onOpen: () => void
  onPlace: (id: string, stretch: Stretch) => void
  onEdit: (id: string, starts_at: number, ends_at: number) => void
  onRename: (id: string, label: string | null) => void
  onRemove: (id: string) => void
  onReorder: (ids: string[]) => void
  busy: boolean
}) {
  const { t } = useTranslation()
  const scale = scaleOf(track.source_duration)
  // The stretch in the hand, where the pointer has it. Drawn on the track and
  // in the seconds under it alike, so the numbers say where it will land
  // before it is let go.
  const [moving, setMoving] = useState<({ id: string } & Stretch) | null>(null)
  const shown = track.cuts.map((cut) =>
    moving?.id === cut.id ? { ...cut, starts_at: moving.starts_at, ends_at: moving.ends_at } : cut,
  )

  // Put in order by a grip rather than by the row: the row is full of fields,
  // and a row that is itself draggable fights them for every press - and
  // HTML5 dragging never starts in a window that takes file drops.
  const order = useReorder({
    order: track.cuts.map((cut) => cut.id),
    onMove: (id, to) => onReorder(orderWithin(splice, track.cuts, id, to)),
    disabled: busy,
  })

  return (
    <section className="flex flex-col gap-1.5">
      <div className="flex min-w-0 items-center gap-2">
        <Film aria-hidden className="size-3.5 shrink-0 text-faint" />
        <Button variant="link" className="min-w-0 truncate text-xs" onClick={onOpen}>
          {track.source_title}
        </Button>
        {track.source_duration !== null && (
          <span className="font-mono text-xs text-faint">
            {formatSeconds(track.source_duration)}
          </span>
        )}
      </div>

      {scale === null ? (
        // A track with no scale would place every cut at an arbitrary point
        // and look exactly as authoritative as a real one.
        <p className="text-sm text-warn">{t('cuts.noLength', { title: track.source_title })}</p>
      ) : (
        <CutTrack
          cuts={shown}
          duration={scale}
          label={t('cuts.track', { title: track.source_title })}
          disabled={busy}
          onMove={(id, stretch, done) => {
            if (!done) {
              setMoving({ id, ...stretch })
              return
            }
            setMoving(null)
            const stored = track.cuts.find((cut) => cut.id === id)
            if (
              stored !== undefined &&
              (stored.starts_at !== stretch.starts_at || stored.ends_at !== stretch.ends_at)
            ) {
              onPlace(id, stretch)
            }
          }}
        />
      )}

      {/* The line where a stretch would land is drawn against this box, so
          it holds the list rather than standing inside it. */}
      <div {...order.listProps} className="relative">
        <ul className="flex flex-col gap-0.5">
          {shown.map((cut) => (
            <li
              key={cut.id}
              {...order.rowProps(cut.id)}
              className={cn(
                'flex items-center gap-2 text-sm',
                order.dragging === cut.id && 'opacity-50',
              )}
            >
              {/* A grip only where there is somewhere to move to. */}
              {track.cuts.length > 1 ? (
                <ReorderGrip {...order.gripProps(cut.id)} title={t('cuts.move')} />
              ) : (
                <span aria-hidden className="w-3.5 shrink-0" />
              )}
              <span className="w-5 shrink-0 text-right font-mono text-xs text-faint">
                {cut.position}
              </span>
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
              <span className="w-12 shrink-0 font-mono text-xs text-faint">
                {formatSeconds(lengthOf(cut))}
              </span>
              {/* The name is stored, so it is shown and typeable: a column the
                schema keeps and no screen draws is how a field dies. */}
              <Label
                value={cut.label}
                onCommit={(label) => onRename(cut.id, label)}
                disabled={busy}
              />
              <Button
                variant="icon"
                size="icon-sm"
                title={t('cuts.remove')}
                aria-label={t('cuts.remove')}
                disabled={busy}
                onClick={() => onRemove(cut.id)}
              >
                <X aria-hidden />
              </Button>
            </li>
          ))}
        </ul>
        <ReorderIndicator offset={order.slotOffset} />
      </div>
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
      className="w-18 shrink-0"
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
    <Panel className="flex flex-wrap items-center gap-2.5 px-3 py-2.5">
      <div className="min-w-50 flex-1">
        <SectionLabel>{t('cuts.shotList')}</SectionLabel>
        {/* Good when it is ready: the one line on the tab that says the
            work is done. */}
        <p className={cn('mt-1 text-sm', blocker === null ? 'text-good' : 'text-dim')}>
          {blocker === null ? t('cuts.readyToCut') : t(`cuts.blocked.${blocker}`)}
        </p>
      </div>
      <Button
        size="sm"
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
