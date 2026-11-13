import { useState, type RefObject } from 'react'
import { useTranslation } from 'react-i18next'
import { CalendarOff } from 'lucide-react'
import type { ScheduledRelease, TimeOff } from '@/lib/api/types'
import type { Ghost } from '@/lib/layout'
import { byDate, today } from '@/lib/month'
import type { Dragging } from '@/lib/useChipDrag'
import { RowContextMenu } from '@/components/RowMenu'
import { BigCalendar, dayOf } from '@/components/ui/big-calendar'
import { CarriedChip } from '@/features/calendar/CarriedChip'
import { LandingLines, landingSays } from '@/features/calendar/LandingLines'
import { PlannedLine, ReleaseLine } from '@/features/calendar/ReleaseLine'
import { QUEUE, type DaysTarget } from '@/features/calendar/useCalendarDrag'
import { useLandingVerdict } from '@/features/calendar/useLandingVerdict'
import { dayActions, mondayFirst } from '@/features/calendar/days'

interface Props {
  /** Any day of the month to draw. */
  month: string
  /** The keyboard walked off the grid: any day of the month it reached. */
  onMonthChange: (date: string) => void
  slots: readonly ScheduledRelease[]
  /** Everything booked, whatever the kind filter shows: who is on a day is
   *  not a question the filter answers. */
  booked: readonly ScheduledRelease[]
  /** The closed days among them, with the span that closes each. */
  closed: ReadonlyMap<string, TimeOff>
  /** An auto-layout preview by day: where the queue would land, booked by
      nothing yet. Drawn as dashed lines among the real ones. */
  ghosts?: Map<string, Ghost[]>
  /** The queued release waiting for a date, if any; pressing a day takes it. */
  claimingId: string | null
  onPickDay: (date: string) => void
  onOpenRelease: (releaseId: string) => void
  /** Add a work that goes out on this day: the `+` a day shows under the
   * pointer, and the first entry in its right-click menu. */
  onAddOn: (date: string) => void
  /** Close the day, or edit the span that closes it. */
  onTimeOff: (date: string, closedBy: TimeOff | null) => void
  /** Shift everything planned from the day on. */
  onShiftFrom: (date: string) => void
  /** The month's edges, for the carried line to turn it (`useCalendarDrag`). */
  gridRef: RefObject<HTMLDivElement | null>
  dragging: Dragging | null
  onGrab: (event: React.PointerEvent, releaseId: string) => void
  /** The day, or the queue, under the pointer. */
  over: string | null
  /** The handlers that find the day under a carried line or a pick. */
  pointer: DaysTarget
}

/**
 * The month: dowel's BigCalendar, with kilna's releases on its days.
 *
 * The grid holds the window's height: its weeks share it and a day never
 * grows past its share, so the screen under it cannot scroll. What a day has
 * no room for folds into a count that opens the day in place; the grid is one
 * tab stop, and the arrows walk its days (ADR 0038, dowel's BigCalendar).
 *
 * The question a release plan is read for is "is that week empty", and the
 * month answers it. Weeks start on Monday in every language: a release plan
 * is read in working weeks (`mondayFirst`).
 *
 * What kilna adds to dowel's month is its own: a closed day, hatched and named
 * in its first line (v0.96); the plan of the auto-layout as dashed lines; the
 * right-click menu of a day; and what landing on a day would mean, said at its
 * foot while something is carried over it.
 */
export function MonthGrid({
  month,
  onMonthChange,
  slots,
  booked,
  closed,
  ghosts,
  claimingId,
  onPickDay,
  onOpenRelease,
  onAddOn,
  onTimeOff,
  onShiftFrom,
  gridRef,
  dragging,
  onGrab,
  over,
  pointer,
}: Props) {
  const { t, i18n } = useTranslation()
  // The day a right click landed on, for the menu it opens.
  const [menuDay, setMenuDay] = useState<string | null>(null)

  const onDays = byDate(slots, (slot) => slot.scheduled_at)
  const now = today()
  const claiming = claimingId !== null
  const carried = slots.find((slot) => slot.id === dragging?.id)

  // What the day under the pointer already holds, what the release would
  // repeat there (ADR 0054) and whether the day is closed - each a warning
  // before it lands, never a refusal (`useLandingVerdict`).
  const moving = dragging?.id ?? claimingId
  const onDay = over !== null && over !== QUEUE
  const verdictFor = useLandingVerdict({
    moving,
    over: onDay ? over : null,
    slots: booked,
    closed,
  })
  const verdict = onDay ? verdictFor(over) : null
  const live = claiming || dragging !== null

  const renderDay = (date: string) => {
    const span = closed.get(date)
    return [
      // Why the day is closed, as its first line: still a day - something can
      // be put on it by hand - but plainly not one the plan meant to fill.
      ...(span === undefined
        ? []
        : [
            <div
              key="closed"
              data-closed
              title={t('timeOff.closedBy', { title: span.label })}
              className="flex h-6 shrink-0 items-center gap-0.5 px-1 text-2xs text-faint"
            >
              <CalendarOff aria-hidden className="size-3 shrink-0" />
              <span className="truncate">{span.label}</span>
            </div>,
          ]),
      // Booked lines first, then the plan's: a booking is a fact, a planned
      // line is the question the bar above the month is asking.
      ...(onDays.get(date) ?? []).map((slot) => (
        <ReleaseLine
          key={slot.id}
          slot={slot}
          date={date}
          now={now}
          dragging={dragging?.id === slot.id}
          onGrab={(event) => onGrab(event, slot.id)}
          onOpen={() => onOpenRelease(slot.id)}
        />
      )),
      ...(ghosts?.get(date) ?? []).map((ghost) => (
        <PlannedLine key={`plan-${ghost.releaseId}`} ghost={ghost} />
      )),
    ]
  }

  const menuSpan = menuDay === null ? null : (closed.get(menuDay) ?? null)

  return (
    <RowContextMenu
      actions={
        menuDay === null
          ? []
          : dayActions(t, menuSpan, {
              add: () => onAddOn(menuDay),
              timeOff: () => onTimeOff(menuDay, menuSpan),
              shift: () => onShiftFrom(menuDay),
            })
      }
      render={
        <div
          ref={gridRef}
          // The day the right click is on, read before the menu opens; off
          // the days - the weekday row - there is nothing to offer.
          onContextMenuCapture={(event) => {
            const day = dayOf(event.target)
            if (day === null) {
              event.preventDefault()
              event.stopPropagation()
            }
            setMenuDay(day)
          }}
          className="flex min-h-0 min-w-0 flex-1 flex-col"
        />
      }
    >
      <BigCalendar
        month={month}
        onMonthChange={onMonthChange}
        locale={mondayFirst(i18n.language)}
        renderDay={renderDay}
        moreLabel={(count) => t('calendar.moreOnDay', { count })}
        fewerLabel={t('calendar.showFewer')}
        onAddDay={onAddOn}
        addDayLabel={t('calendar.addOnDay')}
        // Something is waiting for a date: the days become a way to pick one,
        // rather than a picture of what is booked.
        onDaySelect={claiming ? onPickDay : undefined}
        dropTarget={live && onDay ? over : null}
        dropHint={landingSays(verdict) ? <LandingLines verdict={verdict} /> : undefined}
        // One pair of handlers for both gestures, the queue's pick and a line
        // in the air. The carried line is `pointer-events: none`, so the day
        // underneath keeps receiving the pointer and lights up as it is
        // crossed.
        onPointerOver={live ? pointer.onPointerOver : undefined}
        onPointerLeave={live ? pointer.onPointerLeave : undefined}
        className={`flex-1 ${CLOSED_DAYS}`}
      />

      {dragging !== null && carried !== undefined && (
        <CarriedChip dragging={dragging} slot={carried} now={now} />
      )}
    </RowContextMenu>
  )
}

/**
 * The ground of a closed day: a fine hatch in the hairline's colour, over
 * whatever ground the day already has - so a closed day of a neighbouring
 * month is still quieter than one of this month. The month draws its days,
 * so a closed one is found by the line that names it.
 */
const CLOSED_DAYS =
  '[&_[data-day]:has([data-closed])]:[background-image:repeating-linear-gradient(135deg,var(--line-2)_0_1px,transparent_1px_7px)]'
