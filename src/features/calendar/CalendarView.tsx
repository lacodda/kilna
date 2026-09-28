import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import type { Placement } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { Frame, ListDetail } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { CalendarDialogs, type CalendarDialog } from '@/features/calendar/CalendarDialogs'
import { CalendarHead } from '@/features/calendar/CalendarHead'
import { LayoutPreview } from '@/features/calendar/LayoutPreview'
import { MonthGrid } from '@/features/calendar/MonthGrid'
import { MonthSkeleton } from '@/features/calendar/MonthSkeleton'
import { QueuePane } from '@/features/calendar/QueuePane'
import { useCalendarActions } from '@/features/calendar/useCalendarActions'
import { QUEUE, useCalendarDrag } from '@/features/calendar/useCalendarDrag'
import { filterByKind, filterGhosts, type KindFilter } from '@/lib/calendarFilter'
import { loadLayout, saveLayout, type CalendarLayout } from '@/lib/calendarLayout'
import { ghostsOf } from '@/lib/layout'
import { monthOf, today, type Month } from '@/lib/month'
import { batchable } from '@/lib/releaseFields'

interface Props {
  onSelect: (workId: string) => void
}

/**
 * The queue feeds the calendar: the month on the left, what still needs a
 * date on the right, one head across both. A day holds as many releases as
 * are put on it; dropping onto a taken day says who is there, and does not
 * push back. A release carried off the month onto the queue goes back into
 * it.
 *
 * This file is the screen's state and how its parts are joined. The parts
 * are their own files: the head (`CalendarHead`), the queue (`QueuePane`),
 * the month (`MonthGrid`, a `DayCell` each), the dialogs
 * (`CalendarDialogs`), the writes (`useCalendarActions`) and the carried chip
 * (`useCalendarDrag`).
 */
export function CalendarView({ onSelect }: Props) {
  const [picked, setPicked] = useState<string | null>(null)
  const [month, setMonth] = useState<Month>(() => monthOf(today()))
  // Which kinds the month is showing. The queue is deliberately not filtered
  // with it (`QueuePane`).
  const [kind, setKind] = useState<KindFilter>(null)
  // How much of the screen the month gets. Named for the width rather than
  // the layout because `layout` below is the auto-layout's plan, and two
  // different things under one word is how the wrong one gets read. Loaded
  // once: it is a standing choice, not something to rediscover every visit.
  const [width, setWidth] = useState<CalendarLayout>(loadLayout)
  // The auto-layout plan being previewed, or null. Applying books exactly
  // this array; any other calendar change makes it a picture of the past, so
  // settling clears it.
  const [layout, setLayout] = useState<Placement[] | null>(null)
  // The dialog open, and on what - by id, not by row: holding the row would
  // freeze it at the moment it was opened.
  const [dialog, setDialog] = useState<CalendarDialog>(null)

  const slots = useQuery(queries.calendar())
  const queued = useQuery(queries.releaseQueue())

  const actions = useCalendarActions({
    onChanged: () => setLayout(null),
    // Jumping to the plan's first month is what makes the ghosts visible at
    // all when the queue lands beyond the month on screen.
    onPlanned: (placements) => {
      setLayout(placements)
      const first = placements[0]
      if (first !== undefined) setMonth(monthOf(first.date))
    },
    onClaimed: () => setPicked(null),
    onDayFilled: () => setDialog((current) => (current?.kind === 'fill' ? null : current)),
  })

  const drag = useCalendarDrag({
    month,
    onMonthChange: setMonth,
    // Let go over the day it came from, a chip has not moved: nothing is
    // written, and no toast says it was.
    onMove: (id, date) => {
      const from = slots.data?.find((entry) => entry.id === id)?.scheduled_at
      if (from !== date) actions.move.mutate({ id, date })
    },
    onUnschedule: (id) => actions.unschedule.mutate(id),
  })

  // The planned releases of the month on screen: what the fields batch is
  // about. Read off the unfiltered month - a narrowed view is a way of
  // looking, not an instruction about which releases to write.
  const planned = batchable(
    (slots.data ?? []).filter(
      (entry) =>
        entry.scheduled_at !== null &&
        entry.scheduled_at.slice(0, 7) === `${month.year}-${String(month.month).padStart(2, '0')}`,
    ),
  )

  // The month holds the height it is given at every window width, and its
  // weeks share it; nothing on this screen scrolls but the queue's list.
  const monthView = (
    <Loaded query={slots} fill skeleton={<MonthSkeleton />}>
      {(data) => (
        <MonthGrid
          month={month}
          slots={filterByKind(data, kind)}
          // Filtered with the chips, so a narrowed month does not draw a
          // plan it is not showing. Booking still applies every placement:
          // the filter is a view of the month, not an instruction about
          // what to schedule.
          ghosts={
            layout === null ? undefined : filterGhosts(ghostsOf(layout, queued.data ?? []), kind)
          }
          // A queued release is waiting for a date: the grid becomes a way
          // to pick one, rather than a picture of what is booked.
          claimingId={picked}
          onPickDay={(date) => {
            if (picked !== null) actions.claim.mutate({ id: picked, date })
          }}
          onOpenRelease={(id) => setDialog({ kind: 'edit', releaseId: id })}
          onAddOn={(date) => setDialog({ kind: 'fill', date })}
          gridRef={drag.gridRef}
          dragging={drag.dragging}
          onGrab={drag.begin}
          over={drag.over}
          target={drag.target}
        />
      )}
    </Loaded>
  )

  const carrying = drag.dragging !== null

  return (
    <Frame
      head={
        <div className="flex w-full flex-col gap-2.5">
          <CalendarHead
            month={month}
            onMonthChange={setMonth}
            slots={slots.data ?? []}
            kind={kind}
            onKindChange={setKind}
            planned={planned.length}
            onFillFields={() => setDialog({ kind: 'fields' })}
            fillingFields={actions.fillFields.isPending}
            width={width}
            onWidthChange={(next) => {
              setWidth(next)
              saveLayout(next)
            }}
            // With the queue hidden, the head is the one place left to take
            // a release back.
            drop={
              width === 'full' && carrying
                ? { over: drag.over === QUEUE, target: drag.target(QUEUE) }
                : undefined
            }
          />
          {layout !== null && (
            <LayoutPreview
              placements={layout}
              onCancel={() => setLayout(null)}
              onBook={() => actions.book.mutate(layout)}
              booking={actions.book.isPending}
            />
          )}
        </div>
      }
    >
      {/* Hidden entirely in the full-width layout rather than collapsed: a
          narrow strip of it would still take the width the month is being
          given. Claiming a slot from the queue goes with it - that is what
          the layout is for, and the toggle is one click away. */}
      {width === 'full' ? (
        monthView
      ) : (
        <ListDetail
          side="end"
          detail={monthView}
          list={
            <QueuePane
              queued={queued}
              picked={picked}
              onPick={setPicked}
              onClaim={(id, date) => actions.claim.mutate({ id, date })}
              claiming={actions.claim.isPending}
              onLayOut={layout === null ? () => actions.preview.mutate() : undefined}
              layingOut={actions.preview.isPending}
              carrying={carrying}
              over={drag.over === QUEUE}
              target={drag.target(QUEUE)}
            />
          }
        />
      )}

      <CalendarDialogs
        dialog={dialog}
        onDialog={setDialog}
        slots={slots.data ?? []}
        planned={planned}
        actions={actions}
        onOpenWork={onSelect}
      />
    </Frame>
  )
}
