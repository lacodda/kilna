import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { UseQueryResult } from '@tanstack/react-query'
import type { ScheduledRelease, ScoredWork } from '@/lib/api/types'
import { coverImageFor } from '@/lib/cover'
import { formatNumber } from '@/lib/format'
import { stagesOf } from '@/lib/stages'
import { unplannedWorks } from '@/lib/unplanned'
import { useCovers } from '@/lib/useCovers'
import { allOf, labelOf, say as sayLabel, useProfile } from '@/lib/useProfile'
import { Select } from '@/components/AppSelect'
import { DatePicker } from '@/components/DatePicker'
import { Loaded } from '@/components/Loaded'
import { ReadyMarks } from '@/components/ReadyMarks'
import { WorkRepeatMark } from '@/components/RepeatMark'
import { Pane } from '@/components/frame'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { Input } from '@/components/ui/input'
import { RowButton } from '@/components/ui/list-row'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { SkeletonList } from '@/components/ui/skeleton'
import { QueueDrop } from '@/features/calendar/QueueDrop'
import { RepeatWarning } from '@/features/calendar/RepeatWarning'
import { carriedWork, carryWork, type DropTarget } from '@/features/calendar/useCalendarDrag'
import { cn } from '@/lib/utils'

/** Which list the pane shows: releases waiting for a day, or works with
 *  nothing planned yet. */
export type PaneSource = 'queue' | 'works'

interface Props {
  queued: UseQueryResult<ScheduledRelease[]>
  /** The catalogue's rows, for the works with nothing planned. */
  works: UseQueryResult<ScoredWork[]>
  source: PaneSource
  onSourceChange: (source: PaneSource) => void
  /** What is waiting for a date, if anything: a queued release's id, or a
   *  work's (`carryWork`). */
  picked: string | null
  onPick: (id: string | null) => void
  /** Give the picked one the date typed in the foot. */
  onClaim: (id: string, date: string) => void
  claiming: boolean
  /** Plan the whole queue to the profile's rhythm; absent while a plan is up. */
  onLayOut?: () => void
  layingOut: boolean
  /** A row pressed: the drag hook decides whether it becomes a carry. */
  onGrab: (event: React.PointerEvent, id: string) => void
  /** A booked release is in the air: the queue takes it back. */
  carrying: boolean
  /** It is over the queue right now. */
  over: boolean
  target: DropTarget
}

/**
 * What can still be given a day, beside the calendar it is read against.
 *
 * Two lists under one head (v0.96): the queue - releases waiting for a date,
 * strongest first, as it has always been - and the works with nothing planned
 * yet, the catalogue as the calendar needs it. A row of either is picked and
 * then given a day, or carried onto one: a queued release takes the day, a
 * work gets a release of its first door dated to it, as the day's `+` books
 * one. The search and the filters are one set for both: they ask the same
 * question, "what is closest to going out", of whichever list is up.
 *
 * Its head - how to find one of a few hundred - and its foot stand; the list
 * between them scrolls. The foot is where a booked release comes back: the
 * whole pane takes a chip let go over it, and the foot says so before
 * anything is in the air.
 */
export function QueuePane({
  queued,
  works,
  source,
  onSourceChange,
  picked,
  onPick,
  onClaim,
  claiming,
  onLayOut,
  layingOut,
  onGrab,
  carrying,
  over,
  target,
}: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const covers = useCovers()
  const releaseKinds = allOf(profile.config, 'release_kinds')
  const tiers = allOf(profile.config, 'tiers')

  // The date typed for the picked one, when the day is not simply clicked.
  const [slot, setSlot] = useState('')
  // What the list is narrowed to. A view of the PANE, not of the month: the
  // owner asked to see "what is closest to going out" among four hundred
  // works, and that question is asked here. The month's kind chips
  // deliberately do not reach it - hiding part of what is waiting behind a
  // view of the month would hide work waiting.
  const [query, setQuery] = useState('')
  const [workKind, setWorkKind] = useState<string | null>(null)
  const [stage, setStage] = useState<number | null>(null)

  // `stage` is a percentage and the filter is a stop, so the comparison is
  // "has reached this stop" rather than equality - picking "Polishing" shows
  // what is polishing AND what is past it, which is what "closest to going
  // out" means.
  const matches = (kind: string, reached: number | null, title: string) =>
    (workKind === null || kind === workKind) &&
    (stage === null || (reached ?? -1) >= stage) &&
    title.toLowerCase().includes(query.trim().toLowerCase())

  const shownQueue = (queued.data ?? []).filter((entry) =>
    matches(entry.work_kind, entry.work_stage, entry.work_title),
  )
  const unplanned = unplannedWorks(works.data ?? [], queued.data ?? [], profile.config)
  const shownWorks = unplanned.filter((work) => matches(work.kind, work.stage, work.title))

  // One click plans the whole queue to the profile's rhythm. Without a rhythm
  // there is nothing to pace by, and the button says so instead of hiding.
  const noRhythm = profile.config.rhythm == null
  const canLayOut = source === 'queue' && onLayOut !== undefined && (queued.data?.length ?? 0) > 0

  const pickedWork = picked === null ? null : carriedWork(picked)

  const cover = (workId: string) => (
    <span
      aria-hidden
      className="size-6.5 rounded-sm"
      style={{ background: coverImageFor(workId, covers.get(workId)) }}
    />
  )
  const score = (total: number | null) => (
    <span
      className="w-8 text-right font-mono text-sm text-dim"
      // The score orders the list and drives the auto-layout; since v0.44 it
      // decides nothing about who may have a day.
      title={total === null ? t('calendar.unscored') : undefined}
    >
      {total === null ? '—' : formatNumber(total, 0)}
    </span>
  )

  return (
    <div
      // The whole pane takes a release back, not only its foot: a target the
      // width of a list is one a carried chip cannot miss.
      data-queue-drop
      onPointerEnter={carrying ? target.onPointerEnter : undefined}
      onPointerLeave={carrying ? target.onPointerLeave : undefined}
      className="flex min-h-0 min-w-0 flex-1 flex-col"
    >
      <Pane
        label={t('calendar.queue')}
        bodyClassName="p-1.5"
        className={cn('transition-colors', carrying && 'border-accent', over && 'bg-accent-soft')}
        head={
          <div className="flex w-full flex-col gap-2">
            {/* The two lists across the pane's width, and under them what
                the one up is for, with the layout beside the queue's words:
                beside the segments it was pushed out of a pane this narrow. */}
            <SegmentedControl
              aria-label={t('calendar.paneSource')}
              value={source}
              onValueChange={(next) => {
                onSourceChange(next as PaneSource)
                onPick(null)
              }}
              className="w-full *:flex-1"
            >
              <Segment value="queue">
                {t('calendar.queue')}
                <span className="ml-1 font-mono text-xs text-faint tabular-nums">
                  {queued.data?.length ?? 0}
                </span>
              </Segment>
              <Segment value="works">
                {t('calendar.unplanned')}
                <span className="ml-1 font-mono text-xs text-faint tabular-nums">
                  {unplanned.length}
                </span>
              </Segment>
            </SegmentedControl>
            <div className="flex items-center gap-2">
              <p className="min-w-0 flex-1 text-xs text-faint">
                {source === 'queue' ? t('calendar.queueHint') : t('calendar.unplannedHint')}
              </p>
              {canLayOut && (
                <Button
                  size="sm"
                  variant="soft"
                  className="shrink-0"
                  disabled={noRhythm || layingOut}
                  disabledReason={noRhythm ? t('calendar.layoutNeedsRhythm') : undefined}
                  onClick={onLayOut}
                >
                  {t('calendar.layout')}
                </Button>
              )}
            </div>

            {/* Finding one of a few hundred, and seeing what is nearly ready.
                Both lists are ordered by score, which answers "which is best"
                - not "which is closest to going out", which is what someone
                filling a week is actually asking. */}
            <Input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder={t(
                source === 'queue' ? 'calendar.queueSearch' : 'calendar.unplannedSearch',
              )}
              aria-label={t(
                source === 'queue' ? 'calendar.queueSearch' : 'calendar.unplannedSearch',
              )}
              className="text-xs"
            />
            <div className="flex gap-1.5">
              <Select
                value={workKind ?? ''}
                onChange={(value) => setWorkKind(value === '' ? null : value)}
                placeholder={t('calendar.queueAnyKind')}
                aria-label={t('calendar.queueKind')}
                className="min-w-0 flex-1"
                options={profile.config.work_kinds.map((entry) => ({
                  value: entry.key,
                  label: sayLabel(entry.label),
                }))}
              />
              <Select
                value={stage === null ? '' : String(stage)}
                onChange={(value) => setStage(value === '' ? null : Number(value))}
                placeholder={t('calendar.queueAnyStage')}
                aria-label={t('calendar.queueStage')}
                className="min-w-0 flex-1"
                options={stagesOf(profile.config).map((stop) => ({
                  value: String(stop.percent),
                  label: sayLabel(stop.label),
                }))}
              />
            </div>
          </div>
        }
        foot={
          // A pick waiting for a typed date takes the foot; a chip in the air
          // takes it back, since that is what the foot is for.
          picked !== null && !carrying ? (
            <div className="flex w-full flex-col gap-1.5">
              {/* What the typed day would repeat, before Claim is pressed:
                  the month says it while a chip hovers a day, and a day
                  typed here is the same gesture by other means. A work has
                  no release to ask with yet; it is told when it lands. */}
              {pickedWork === null && <RepeatWarning releaseId={picked} day={slot} />}
              <form
                className="flex w-full gap-2"
                onSubmit={(event) => {
                  event.preventDefault()
                  if (slot !== '') onClaim(picked, slot)
                }}
              >
                <DatePicker
                  value={slot}
                  onChange={setSlot}
                  placeholder={t('calendar.slotDate')}
                  aria-label={t('calendar.slotDate')}
                />
                <Button type="submit" variant="primary" disabled={slot === '' || claiming}>
                  {t('calendar.claim')}
                </Button>
              </form>
            </div>
          ) : (
            <QueueDrop active={carrying} over={over} />
          )
        }
      >
        {source === 'queue' ? (
          <Loaded
            query={queued}
            skeleton={<SkeletonList rows={4} />}
            isEmpty={(data) => data.length === 0}
            emptyState={<EmptyState plain title={t('calendar.queueEmpty')} className="p-2" />}
            plain
          >
            {() =>
              shownQueue.length === 0 ? (
                // Plenty waiting, none of it matching: the way out is the
                // search and the two filters right above.
                <EmptyState
                  plain
                  variant="filtered"
                  title={t('calendar.queueNoMatch')}
                  className="p-2"
                />
              ) : (
                <ul className="flex flex-col gap-0.5">
                  {shownQueue.map((entry) => (
                    <li key={entry.id}>
                      <RowButton
                        selected={entry.id === picked}
                        onClick={() => onPick(entry.id === picked ? null : entry.id)}
                        // Carried onto a day, it takes that day: the same as
                        // picking it and clicking the day, in one gesture.
                        onPointerDown={(event) => onGrab(event, entry.id)}
                        className="px-2"
                        // The work's cover, as on the month: the colour a
                        // release has in the queue is the colour its chip will
                        // have once it has a day.
                        start={cover(entry.work_id)}
                        // What goes out, and how the work was judged - the
                        // mockup's line under the title.
                        description={[
                          labelOf(releaseKinds, entry.kind),
                          entry.tier === null ? null : labelOf(tiers, entry.tier),
                        ]
                          .filter((part) => part !== null)
                          .join(' · ')}
                        end={
                          <>
                            {/* Whether its song repeats one out or booked
                                (ADR 0054): picking what goes out next is
                                where a repeat is worth knowing. */}
                            <WorkRepeatMark workId={entry.work_id} />
                            {/* No date yet, so no deadline: the gaps show, calmly. */}
                            <ReadyMarks release={entry} daysLeft={null} />
                            {score(entry.total)}
                          </>
                        }
                      >
                        {entry.work_title}
                      </RowButton>
                    </li>
                  ))}
                </ul>
              )
            }
          </Loaded>
        ) : (
          <Loaded
            query={works}
            skeleton={<SkeletonList rows={4} />}
            isEmpty={() => unplanned.length === 0}
            emptyState={<EmptyState plain title={t('calendar.unplannedEmpty')} className="p-2" />}
            plain
          >
            {() =>
              shownWorks.length === 0 ? (
                <EmptyState
                  plain
                  variant="filtered"
                  title={t('calendar.queueNoMatch')}
                  className="p-2"
                />
              ) : (
                <ul className="flex flex-col gap-0.5">
                  {shownWorks.map((work) => {
                    const id = carryWork(work.work_id)
                    return (
                      <li key={work.work_id}>
                        <RowButton
                          selected={id === picked}
                          onClick={() => onPick(id === picked ? null : id)}
                          onPointerDown={(event) => onGrab(event, id)}
                          className="px-2"
                          start={cover(work.work_id)}
                          description={[
                            labelOf(profile.config.work_kinds, work.kind),
                            work.tier === null ? null : labelOf(tiers, work.tier),
                          ]
                            .filter((part) => part !== null)
                            .join(' · ')}
                          end={
                            <>
                              <WorkRepeatMark workId={work.work_id} />
                              {score(work.total)}
                            </>
                          }
                        >
                          {work.title}
                        </RowButton>
                      </li>
                    )
                  })}
                </ul>
              )
            }
          </Loaded>
        )}
      </Pane>
    </div>
  )
}
