import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { UseQueryResult } from '@tanstack/react-query'
import { CalendarClock } from 'lucide-react'
import type { ScheduledRelease, ScoredWork, ShownVibe, WorkPlan } from '@/lib/api/types'
import { feels } from '@/lib/catalogue'
import { coverImageFor } from '@/lib/cover'
import { formatDay, formatNumber } from '@/lib/format'
import { addDays, today } from '@/lib/month'
import { stageAt, standsAt } from '@/lib/stages'
import { planShown, unplannedWorks } from '@/lib/unplanned'
import { axesOf } from '@/lib/vibe'
import { useCovers } from '@/lib/useCovers'
import { allOf, labelOf, say as sayLabel, useProfile } from '@/lib/useProfile'
import { Select } from '@/components/AppSelect'
import { DatePicker } from '@/components/DatePicker'
import { Loaded } from '@/components/Loaded'
import { ReadyMarks } from '@/components/ReadyMarks'
import { RepeatWarning } from '@/components/RepeatWarning'
import { WorkRepeatMark } from '@/components/RepeatMark'
import { StageChecks } from '@/components/StageChecks'
import { StageDial } from '@/components/StageDial'
import { VibeChecks } from '@/components/VibeChecks'
import { WorkVibeMark } from '@/components/WorkVibeMark'
import { Pane } from '@/components/ui/frame'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { FilterPopover } from '@/components/ui/filter-popover'
import { Input } from '@/components/ui/input'
import { RowButton } from '@/components/ui/list-row'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { SkeletonList } from '@/components/ui/skeleton'
import { QueueDrop } from '@/features/calendar/QueueDrop'
import type { PlanShown } from '@/features/calendar/WorkPlanEditor'
import { carriedWork, carryWork, type DropTarget } from '@/features/calendar/useCalendarDrag'
import { cn } from '@/lib/utils'

/** What each list is for, said under the segments. */
const HINTS = {
  queue: 'calendar.queueHint',
  plans: 'calendar.plansHint',
  works: 'calendar.unplannedHint',
} as const

/** What the search box of each list asks for. */
const SEARCHES = {
  queue: 'calendar.queueSearch',
  plans: 'calendar.plansSearch',
  works: 'calendar.unplannedSearch',
} as const

/** Which list the pane shows: releases waiting for a day, the open planned
 *  slots of works (v0.169), or works with nothing planned yet. */
export type PaneSource = 'queue' | 'plans' | 'works'

interface Props {
  queued: UseQueryResult<ScheduledRelease[]>
  /** The catalogue's rows, for the works with nothing planned. */
  works: UseQueryResult<ScoredWork[]>
  /** The works planned on a day: they stand on the calendar already. */
  plans: readonly WorkPlan[]
  /** A planned slot pressed: its editor opens, as from the calendar. */
  onOpenPlan: (shown: PlanShown) => void
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
 * Three lists under one head: the releases waiting for a date, strongest
 * first, as it has always been; the plans (v0.169, FR-11 11.3) - the open
 * planned slots of works, nearest first, each opening its editor; and the
 * works with nothing planned yet (v0.96), the catalogue as the calendar
 * needs it. A row of either is picked and
 * then given a day, or carried onto one: a queued release takes the day, a
 * work gets a release of its first door dated to it, as the day's `+` books
 * one, and a song - which goes out only as what is made from it - is planned
 * on the day (v0.150.1). The search and the filters are one set for both: they ask the same
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
  plans,
  onOpenPlan,
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
  // The stops ticked, as the catalogue's stage funnel ticks them - the same
  // five dials, read by the same rule (`standsAt`). "What is closest to
  // going out" is the last stops ticked together. Until v0.111 this was a
  // dropdown of the pane's own that meant "has reached".
  const [stages, setStages] = useState<number[]>([])
  // The words of a vibe ticked (v0.169), as the catalogue's funnel ticks
  // them: a week is filled by what it should feel like as much as by what
  // is ready.
  const [vibes, setVibes] = useState<string[]>([])
  const axes = axesOf(profile.config)

  const ticked = stages.length === 0 ? null : new Set(stages)
  const felt = vibes.length === 0 ? null : new Set(vibes)
  const matches = (
    kind: string,
    stage: number,
    title: string,
    vibe: ShownVibe | null | undefined,
  ) =>
    (workKind === null || kind === workKind) &&
    (ticked === null || standsAt(stage, ticked)) &&
    (felt === null || feels(vibe?.vibe, felt)) &&
    title.toLowerCase().includes(query.trim().toLowerCase())

  const shownQueue = (queued.data ?? []).filter((entry) =>
    matches(entry.work_kind, entry.work_stage, entry.work_title, entry.work_vibe),
  )
  const unplanned = unplannedWorks(works.data ?? [], queued.data ?? [], plans, profile.config)
  const shownWorks = unplanned.filter((work) =>
    matches(work.kind, work.stage, work.title, work.vibe),
  )
  // The open slots with their works, nearest first; one whose work the
  // catalogue has not answered for yet is not listed until it has.
  const lead = Math.max(1, profile.config.rhythm?.plan_lead_days ?? 7)
  const now = today()
  const planned = plans
    .flatMap((plan) => {
      const row = works.data?.find((one) => one.work_id === plan.work_id)
      return row === undefined ? [] : [planShown(plan, row)]
    })
    .sort(
      (a, b) =>
        a.plan.scheduled_at.localeCompare(b.plan.scheduled_at) || a.title.localeCompare(b.title),
    )
  const shownPlans = planned.filter((one) => matches(one.kind, one.stage, one.title, one.vibe))

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
                {t('calendar.queueReleases')}
                <span className="ml-1 font-mono text-xs text-dim tabular-nums">
                  {queued.data?.length ?? 0}
                </span>
              </Segment>
              <Segment value="plans">
                {t('calendar.queuePlans')}
                <span className="ml-1 font-mono text-xs text-dim tabular-nums">
                  {planned.length}
                </span>
              </Segment>
              <Segment value="works">
                {t('calendar.unplanned')}
                <span className="ml-1 font-mono text-xs text-dim tabular-nums">
                  {unplanned.length}
                </span>
              </Segment>
            </SegmentedControl>
            <div className="flex items-center gap-2">
              <p className="min-w-0 flex-1 text-xs text-dim">{t(HINTS[source])}</p>
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
              placeholder={t(SEARCHES[source])}
              aria-label={t(SEARCHES[source])}
              className="text-xs"
            />
            <div className="flex items-center gap-1.5">
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
              {/* The catalogue's stage funnel, here beside the kind: filled
                  while it narrows, so a short list does not read as a short
                  queue. It opens towards the pane, which stands at the
                  window's edge. */}
              <FilterPopover
                title={t('calendar.queueStage')}
                label={t('calendar.queueStageFilter')}
                active={ticked !== null}
                clearLabel={t('catalogue.clear')}
                onClear={() => setStages([])}
                align="end"
                className="size-7"
              >
                <StageChecks chosen={stages} onChange={setStages} />
              </FilterPopover>
              {/* By what a work feels like (v0.169): only where the profile
                  words the three lines. */}
              {axes !== undefined && (
                <FilterPopover
                  title={t('calendar.queueVibe')}
                  label={t('calendar.queueVibeFilter')}
                  active={felt !== null}
                  clearLabel={t('catalogue.clear')}
                  onClear={() => setVibes([])}
                  align="end"
                  className="size-7"
                >
                  <VibeChecks axes={axes} chosen={vibes} onChange={setVibes} />
                </FilterPopover>
              )}
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
                  typed here is the same gesture by other means - for a
                  queued release and for a work with none yet alike. */}
              <RepeatWarning
                landing={pickedWork === null ? { release: picked } : { work: pickedWork }}
                day={slot}
              />
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
                            {/* What it feels like - its own, or its song's. */}
                            <WorkVibeMark shown={entry.work_vibe} />
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
        ) : source === 'plans' ? (
          <Loaded
            query={works}
            skeleton={<SkeletonList rows={4} />}
            isEmpty={() => planned.length === 0}
            emptyState={<EmptyState plain title={t('calendar.plansEmpty')} className="p-2" />}
            plain
          >
            {() =>
              shownPlans.length === 0 ? (
                <EmptyState
                  plain
                  variant="filtered"
                  title={t('calendar.queueNoMatch')}
                  className="p-2"
                />
              ) : (
                <ul className="flex flex-col gap-0.5">
                  {shownPlans.map((shown) => {
                    const due = addDays(shown.plan.scheduled_at, -lead) <= now
                    return (
                      <li key={shown.plan.id}>
                        <RowButton
                          onClick={() => onOpenPlan(shown)}
                          className="px-2"
                          start={cover(shown.plan.work_id)}
                          // The day it is promised for, and what it is.
                          description={[
                            formatDay(shown.plan.scheduled_at),
                            labelOf(profile.config.work_kinds, shown.kind),
                          ].join(' · ')}
                          end={
                            <>
                              <WorkRepeatMark workId={shown.plan.work_id} />
                              <StageDial stage={stageAt(profile.config, shown.stage)} />
                              <WorkVibeMark shown={shown.vibe} />
                              <CalendarClock
                                aria-label={due ? t('workPlan.due') : t('workPlan.marker')}
                                className={cn('size-3.5', due ? 'text-warn' : 'text-faint')}
                              />
                            </>
                          }
                        >
                          {shown.title}
                        </RowButton>
                      </li>
                    )
                  })}
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
                              <WorkVibeMark shown={work.vibe} />
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
