import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { Placement, ScheduledRelease, ShiftPlan, TimeOff } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { CarriedLabel } from '@/components/CarriedLabel'
import { Frame, ListDetail } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { AgendaList } from '@/features/calendar/AgendaList'
import { CalendarDialogs, type CalendarDialog } from '@/features/calendar/CalendarDialogs'
import { CalendarHead } from '@/features/calendar/CalendarHead'
import { DayGrid } from '@/features/calendar/DayGrid'
import { LayoutPreview } from '@/features/calendar/LayoutPreview'
import { MonthSkeleton } from '@/features/calendar/MonthSkeleton'
import { QueuePane, type PaneSource } from '@/features/calendar/QueuePane'
import { ShiftPreview } from '@/features/calendar/ShiftPreview'
import { useCalendarActions } from '@/features/calendar/useCalendarActions'
import { QUEUE, carriedWork, useCalendarDrag } from '@/features/calendar/useCalendarDrag'
import { filterByKind, filterGhosts, type KindFilter } from '@/lib/calendarFilter'
import { loadLayout, saveLayout, type CalendarLayout } from '@/lib/calendarLayout'
import {
  daysOf,
  inPeriod,
  loadView,
  periodOf,
  saveView,
  stepAnchor,
  type CalendarView as View,
} from '@/lib/calendarPeriod'
import { ghostsOf, shiftGhostsOf, type Ghost } from '@/lib/layout'
import { today } from '@/lib/month'
import { batchable } from '@/lib/releaseFields'
import { closedDays } from '@/lib/timeOff'
import { say } from '@/lib/toast'
import { useProfile, vocabularyOf } from '@/lib/useProfile'

interface Props {
  onSelect: (workId: string) => void
}

/**
 * A plan waiting for an answer: the auto-layout's placements for the queue,
 * or a shift's moves for what is already booked. One at a time - each is a
 * picture of the calendar, and two pictures over one month answer nothing.
 */
type Preview =
  | { kind: 'layout'; placements: Placement[] }
  | { kind: 'shift'; plan: ShiftPlan; from: string; days: number }
  | null

/**
 * The queue feeds the calendar: the plan on the left, what can still be
 * given a day on the right, one head across both. A day holds as many
 * releases as are put on it; dropping onto a taken day says who is there,
 * and does not push back. A release carried off the calendar onto the queue
 * goes back into it.
 *
 * Since v0.96 the plan is looked at three ways - the month, a week, an agenda
 * of the month - around one day the screen holds (`lib/calendarPeriod`); some
 * days are closed (time off) and the layout steps over them; the booked plan
 * can be shifted, and a period's dates kept, in one gesture each; and the
 * pane beside it lists the works with nothing planned, to carry onto a day.
 *
 * This file is the screen's state and how its parts are joined. The parts
 * are their own files: the head (`CalendarHead`), the pane (`QueuePane`), the
 * days (`DayGrid` for the month and the week, `AgendaList` for the agenda), the
 * dialogs (`CalendarDialogs`), the writes (`useCalendarActions`) and the
 * carried chip (`useCalendarDrag`).
 */
export function CalendarView({ onSelect }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  // What is waiting for a day: a queued release's id, or a work's.
  const [picked, setPicked] = useState<string | null>(null)
  // The day the screen is on. The views show its month or its week.
  const [anchor, setAnchor] = useState(today)
  const [view, setView] = useState<View>(loadView)
  const [source, setSource] = useState<PaneSource>('queue')
  // Which kinds the calendar is showing. The pane is deliberately not
  // filtered with it (`QueuePane`).
  const [kind, setKind] = useState<KindFilter>(null)
  // How much of the screen the calendar gets. Named for the width rather
  // than the layout because the auto-layout's plan is a layout too, and two
  // different things under one word is how the wrong one gets read. Loaded
  // once: it is a standing choice, not something to rediscover every visit.
  const [width, setWidth] = useState<CalendarLayout>(loadLayout)
  // The plan being previewed. Approving books exactly this; any other
  // calendar change makes it a picture of the past, so settling clears it.
  const [preview, setPreview] = useState<Preview>(null)
  // The dialog open, and on what - by id, not by row: holding the row would
  // freeze it at the moment it was opened.
  const [dialog, setDialog] = useState<CalendarDialog>(null)

  const slots = useQuery(queries.calendar())
  const queued = useQuery(queries.releaseQueue())
  const works = useQuery(queries.catalogue())
  const timeOff = useQuery(queries.timeOff())

  const actions = useCalendarActions({
    onChanged: () => setPreview(null),
    // Jumping to the plan's first day is what makes the ghosts visible at
    // all when the queue lands beyond the period on screen.
    onPlanned: (placements) => {
      setPreview({ kind: 'layout', placements })
      const first = placements[0]
      if (first !== undefined) setAnchor(first.date)
    },
    onShiftPlanned: (plan, from, days) => {
      setDialog((current) => (current?.kind === 'shift' ? null : current))
      if (plan.moves.length === 0) {
        say.info(t('shift.nothing', { from }))
        return
      }
      setPreview({ kind: 'shift', plan, from, days })
      const first = plan.moves[0]
      if (first !== undefined && !inPeriod(first.to, periodOf(view, anchor))) {
        setAnchor(first.to)
      }
    },
    onClaimed: () => setPicked(null),
    onDayFilled: () => {
      setPicked(null)
      setDialog((current) => (current?.kind === 'fill' ? null : current))
    },
  })

  /** Book a work on a day: a release of its first door, dated to it - what
   *  the day's `+` does, and what carrying a work onto a day does. */
  const fillDay = (workId: string, date: string) => {
    const work = works.data?.find((row) => row.work_id === workId)
    const door =
      work === undefined ? undefined : vocabularyOf(profile.config, work.kind).release_kinds[0]
    if (work === undefined || door === undefined) {
      say.warn(t('calendar.noDoor', { title: work?.title ?? '' }))
      return
    }
    actions.fillDay.mutate({ work_id: workId, kind: door.key, scheduled_at: date })
  }

  /** Give whatever is picked or carried this day. */
  const land = (id: string, date: string) => {
    const work = carriedWork(id)
    if (work !== null) {
      fillDay(work, date)
      return
    }
    const booked = slots.data?.find((entry) => entry.id === id)
    if (booked !== undefined) {
      // Let go over the day it came from, a chip has not moved: nothing is
      // written, and no toast says it was.
      if (booked.scheduled_at !== date) actions.move.mutate({ id, date })
      return
    }
    actions.claim.mutate({ id, date })
  }

  const drag = useCalendarDrag({
    onStep: (step) => setAnchor((current) => stepAnchor(view, current, step)),
    onMove: land,
    // Only a booked release goes back to the queue: a queued one is there
    // already, and a work has nothing to take off the calendar.
    onUnschedule: (id) => {
      if (slots.data?.some((entry) => entry.id === id) === true) actions.unschedule.mutate(id)
    },
  })

  const period = periodOf(view, anchor)
  const days = daysOf(view, anchor)
  const closed = closedDays(
    timeOff.data ?? [],
    days.map((day) => day.date),
  )

  // The planned releases of the period on screen: what the fields batch is
  // about. Read off the unfiltered calendar - a narrowed view is a way of
  // looking, not an instruction about which releases to write.
  const planned = batchable(
    (slots.data ?? []).filter(
      (entry) => entry.scheduled_at !== null && inPeriod(entry.scheduled_at, period),
    ),
  )

  // The plan on screen, as ghosts by day: where the queue would land, or
  // where a shift would put what is booked. Filtered with the chips, so a
  // narrowed view does not draw a plan it is not showing; booking still
  // applies every move - the filter is a view, not an instruction.
  const ghostsFor = (data: readonly ScheduledRelease[]): Map<string, Ghost[]> | undefined => {
    if (preview === null) return undefined
    const ghosts =
      preview.kind === 'layout'
        ? ghostsOf(preview.placements, queued.data ?? [])
        : shiftGhostsOf(preview.plan.moves, data)
    return filterGhosts(ghosts, kind)
  }

  // What a carried row of the pane is, said beside the pointer: a row is the
  // width of the pane, and drawn whole it would cover the days it is for.
  const carriedTitle = (() => {
    const id = drag.dragging?.id
    if (id === undefined) return null
    const work = carriedWork(id)
    if (work !== null) return works.data?.find((row) => row.work_id === work)?.title ?? null
    if (slots.data?.some((entry) => entry.id === id) === true) return null
    return queued.data?.find((entry) => entry.id === id)?.work_title ?? null
  })()

  const onTimeOff = (date: string, span: TimeOff | null) =>
    setDialog({
      kind: 'timeOff',
      start: span === null ? { day: date } : { editing: span.id },
    })
  const onShiftFrom = (date: string) => setDialog({ kind: 'shift', start: { from: date, days: 7 } })

  // The grid holds the height it is given at every window width, and its
  // weeks share it; nothing on this screen scrolls but the pane's list and
  // the agenda.
  const plan = (
    <Loaded query={slots} fill skeleton={<MonthSkeleton />}>
      {(data) => {
        const shared = {
          slots: filterByKind(data, kind),
          booked: data,
          ghosts: ghostsFor(data),
          closed,
          // Something is waiting for a date: the days become a way to pick
          // one, rather than a picture of what is booked.
          claimingId: picked,
          onPickDay: (date: string) => {
            if (picked !== null) land(picked, date)
          },
          onOpenRelease: (id: string) => setDialog({ kind: 'edit', releaseId: id }),
          onAddOn: (date: string) => setDialog({ kind: 'fill', date }),
          onTimeOff,
          onShiftFrom,
          dragging: drag.dragging,
          onGrab: drag.begin,
          over: drag.over,
          target: drag.target,
        }
        return view === 'agenda' ? (
          <AgendaList days={days} {...shared} />
        ) : (
          <DayGrid days={days} roomy={view === 'week'} gridRef={drag.gridRef} {...shared} />
        )
      }}
    </Loaded>
  )

  const carrying = drag.dragging !== null
  // A booked release in the air can go back to the queue; a row from the
  // pane is already off the calendar.
  const carryingBooked =
    carrying && slots.data?.some((entry) => entry.id === drag.dragging?.id) === true

  return (
    <Frame
      head={
        <div className="flex w-full flex-col gap-2.5">
          <CalendarHead
            view={view}
            onViewChange={(next) => {
              setView(next)
              saveView(next)
            }}
            anchor={anchor}
            onAnchorChange={setAnchor}
            slots={slots.data ?? []}
            kind={kind}
            onKindChange={setKind}
            planned={planned.length}
            onFillFields={() => setDialog({ kind: 'fields' })}
            fillingFields={actions.fillFields.isPending}
            onTimeOff={() => setDialog({ kind: 'timeOff', start: {} })}
            onShift={() => onShiftFrom(today())}
            onPinPeriod={(pinned) => actions.pinPeriod.mutate({ ...period, pinned })}
            pinning={actions.pinPeriod.isPending}
            width={width}
            onWidthChange={(next) => {
              setWidth(next)
              saveLayout(next)
            }}
            // With the pane hidden, the head is the one place left to take
            // a release back.
            drop={
              width === 'full' && carryingBooked
                ? { over: drag.over === QUEUE, target: drag.target(QUEUE) }
                : undefined
            }
          />
          {preview?.kind === 'layout' && (
            <LayoutPreview
              placements={preview.placements}
              onCancel={() => setPreview(null)}
              onBook={() => actions.book.mutate(preview.placements)}
              booking={actions.book.isPending}
            />
          )}
          {preview?.kind === 'shift' && (
            <ShiftPreview
              plan={preview.plan}
              from={preview.from}
              days={preview.days}
              onCancel={() => setPreview(null)}
              onShift={() => actions.shift.mutate(preview.plan.moves)}
              shifting={actions.shift.isPending}
            />
          )}
        </div>
      }
    >
      {/* Hidden entirely in the full-width layout rather than collapsed: a
          narrow strip of it would still take the width the calendar is being
          given. Claiming a slot from the pane goes with it - that is what the
          layout is for, and the toggle is one click away. */}
      {width === 'full' ? (
        plan
      ) : (
        <ListDetail
          side="end"
          detail={plan}
          list={
            <QueuePane
              queued={queued}
              works={works}
              source={source}
              onSourceChange={setSource}
              picked={picked}
              onPick={setPicked}
              onClaim={land}
              claiming={actions.claim.isPending || actions.fillDay.isPending}
              onLayOut={preview === null ? () => actions.preview.mutate() : undefined}
              layingOut={actions.preview.isPending}
              onGrab={drag.begin}
              carrying={carryingBooked}
              over={drag.over === QUEUE}
              target={drag.target(QUEUE)}
            />
          }
        />
      )}

      {drag.dragging !== null && carriedTitle !== null && (
        <CarriedLabel pointer={drag.dragging.pointer} label={carriedTitle} />
      )}

      <CalendarDialogs
        dialog={dialog}
        onDialog={setDialog}
        slots={slots.data ?? []}
        planned={planned}
        spans={timeOff.data ?? []}
        actions={actions}
        onOpenWork={onSelect}
      />
    </Frame>
  )
}
