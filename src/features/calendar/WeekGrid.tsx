import { useEffect, useRef, useState, type RefObject } from 'react'
import { useTranslation } from 'react-i18next'
import type { ScheduledRelease, TimeOff } from '@/lib/api/types'
import type { Ghost } from '@/lib/layout'
import { byDate, today, type Day } from '@/lib/month'
import type { Dragging } from '@/lib/useChipDrag'
import { linesThatFit } from '@/components/ui/big-calendar'
import { weekdayNames } from '@/components/ui/calendar-math'
import { CarriedChip } from '@/features/calendar/CarriedChip'
import { mondayFirst } from '@/features/calendar/days'
import { WeekDay } from '@/features/calendar/WeekDay'
import { QUEUE, type DaysTarget } from '@/features/calendar/useCalendarDrag'
import { useLandingVerdict } from '@/features/calendar/useLandingVerdict'

interface Props {
  /** The seven days of the week, Monday first. */
  days: readonly Day[]
  slots: readonly ScheduledRelease[]
  /** Everything booked, whatever the kind filter shows: who is on a day is
   *  not a question the filter answers. */
  booked: readonly ScheduledRelease[]
  /** The closed days among them, with the span that closes each. */
  closed: ReadonlyMap<string, TimeOff>
  /** An auto-layout preview by day: where the queue would land, booked by
      nothing yet. Drawn as dashed lines among the real ones. */
  ghosts?: Map<string, Ghost[]>
  /** The queued release waiting for a date, if any; clicking a day takes it. */
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
  /** The week's edges, for the carried line to turn it (`useCalendarDrag`). */
  gridRef: RefObject<HTMLDivElement | null>
  dragging: Dragging | null
  onGrab: (event: React.PointerEvent, releaseId: string) => void
  /** The day, or the queue, under the pointer. */
  over: string | null
  /** The handlers that find the day under a carried line or a pick. */
  pointer: DaysTarget
}

/**
 * The week (v0.96): seven days in one row, each with a month's five rows of
 * room - everything a busy Friday holds.
 *
 * Drawn here rather than by dowel's BigCalendar, which draws a month: the
 * days are the month's - the same lines, the same fold into a count, the same
 * hatch on a closed day - and the grid holds the window's height the same
 * way, so the screen under it cannot scroll. Its title and arrows are the
 * screen's head, one row with the filters and the actions.
 */
export function WeekGrid({
  days,
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
  const { i18n } = useTranslation()

  // How many lines a day has room for. Every day is the same height, so one
  // measurement answers for all of them; it is taken again whenever the
  // window, the queue beside the week, or the banner above it changes the
  // week's size.
  const cellsRef = useRef<HTMLDivElement>(null)
  const [lines, setLines] = useState(UNMEASURED_LINES)
  useEffect(() => {
    const cells = cellsRef.current
    if (cells === null || typeof ResizeObserver === 'undefined') return
    const observer = new ResizeObserver(() => {
      const area = cells.querySelector<HTMLElement>('[data-day-lines]')
      if (area !== null) setLines(linesThatFit(area.clientHeight))
    })
    observer.observe(cells)
    return () => observer.disconnect()
  }, [])

  // The one day showing everything it holds, if any. One at a time, as the
  // month opens its days.
  const [expanded, setExpanded] = useState<string | null>(null)

  const carried = slots.find((slot) => slot.id === dragging?.id)
  const onDays = byDate(slots, (slot) => slot.scheduled_at)
  const now = today()
  const claiming = claimingId !== null
  const live = claiming || dragging !== null

  const moving = dragging?.id ?? claimingId
  const onDay = over !== null && over !== QUEUE
  const verdictFor = useLandingVerdict({
    moving,
    over: onDay ? over : null,
    slots: booked,
    closed,
  })

  return (
    <div
      ref={gridRef}
      onPointerOver={live ? pointer.onPointerOver : undefined}
      onPointerLeave={live ? pointer.onPointerLeave : undefined}
      className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden rounded-lg border border-line bg-raise"
    >
      <div className="grid shrink-0 grid-cols-7 border-b border-line">
        {weekdayNames(mondayFirst(i18n.language), 1).map((name) => (
          <div key={name} className="caption truncate px-2 py-1.5">
            {name}
          </div>
        ))}
      </div>

      <div ref={cellsRef} className="grid min-h-0 flex-1 grid-cols-7">
        {days.map((day, index) => (
          <WeekDay
            key={day.date}
            day={day}
            releases={onDays.get(day.date) ?? []}
            ghosts={ghosts?.get(day.date) ?? []}
            now={now}
            lines={lines}
            expanded={expanded === day.date}
            onExpand={(open) => setExpanded(open ? day.date : null)}
            claiming={claiming}
            onPick={() => onPickDay(day.date)}
            onAdd={() => onAddOn(day.date)}
            closed={closed.get(day.date) ?? null}
            onTimeOff={() => onTimeOff(day.date, closed.get(day.date) ?? null)}
            onShiftFrom={() => onShiftFrom(day.date)}
            onOpenRelease={onOpenRelease}
            carrying={dragging?.id ?? null}
            onGrab={onGrab}
            over={live && over === day.date}
            verdict={verdictFor(day.date)}
            lastColumn={index % 7 === 6}
          />
        ))}
      </div>

      {dragging !== null && carried !== undefined && (
        <CarriedChip dragging={dragging} slot={carried} now={now} />
      )}
    </div>
  )
}

/* What a day is assumed to hold before it has been measured - the first
 * frame, and every test, since jsdom lays nothing out. */
const UNMEASURED_LINES = 3
