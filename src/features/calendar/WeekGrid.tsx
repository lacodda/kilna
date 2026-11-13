import { useEffect, useRef, useState, type RefObject } from 'react'
import { useTranslation } from 'react-i18next'
import type { ScheduledRelease, TimeOff } from '@/lib/api/types'
import type { Ghost } from '@/lib/layout'
import { byDate, today, type Day } from '@/lib/month'
import { CHIP_HEIGHT, linesThatFit, ROOMY_CHIP_HEIGHT, UNMEASURED_LINES } from '@/lib/calendarDay'
import type { Dragging } from '@/lib/useChipDrag'
import { CarriedChip } from '@/features/calendar/CarriedChip'
import { DayCell } from '@/features/calendar/DayCell'
import type { DropTarget } from '@/features/calendar/useCalendarDrag'
import { useLandingVerdict } from '@/features/calendar/useLandingVerdict'

interface Props {
  /** The days to draw, a week to a row: a month's whole weeks, or one week. */
  days: readonly Day[]
  /** Two-line chips, for a week: its days have the height (v0.96). */
  roomy?: boolean
  slots: readonly ScheduledRelease[]
  /** Everything booked, whatever the kind filter shows: who is on a day is
   *  not a question the filter answers. */
  booked: readonly ScheduledRelease[]
  /** The closed days among them, with the span that closes each. */
  closed: ReadonlyMap<string, TimeOff>
  /** An auto-layout preview by day: where the queue would land, booked by
      nothing yet. Drawn as dashed chips among the real ones. */
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
  /** The month's edges, for the carried chip to turn it (`useCalendarDrag`). */
  gridRef: RefObject<HTMLDivElement | null>
  dragging: Dragging | null
  onGrab: (event: React.PointerEvent, releaseId: string) => void
  /** The day, or the queue, under the pointer. */
  over: string | null
  target: (key: string) => DropTarget
}

const WEEKDAYS = ['mon', 'tue', 'wed', 'thu', 'fri', 'sat', 'sun'] as const

/**
 * Days a week to a row, with what is booked on each: the month, and since
 * v0.96 the week - the same seven columns, one row tall.
 *
 * The flat list this replaced answered "what is scheduled" but never "is that
 * week empty" - which is the question a release plan is actually read for.
 *
 * The grid holds the window's height: its weeks share it (`auto-rows-fr`)
 * and a day never grows past its share, so the grid does not scroll and the
 * screen under it cannot. What a day has no room for folds into a count
 * (`DayCell`). A week is one row, so each of its days has a month's five
 * rows of room: everything a busy Friday holds, and chips wide enough for
 * their covers. Its title and arrows are the screen's head since v0.79, one
 * row with the filters and the actions, as the mockup draws it.
 */
export function DayGrid({
  days,
  roomy = false,
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
  target,
}: Props) {
  const { t } = useTranslation()

  // How many chip lines a day has room for. Every day of the month is the
  // same height, so one measurement answers for all of them; it is taken
  // again whenever the window, the queue beside the month, or the banner above
  // it changes the month's size.
  const cellsRef = useRef<HTMLDivElement>(null)
  const [lines, setLines] = useState(UNMEASURED_LINES)
  useEffect(() => {
    const cells = cellsRef.current
    if (cells === null) return
    const measure = () => {
      const area = cells.querySelector<HTMLElement>('[data-day-lines]')
      if (area !== null) {
        setLines(linesThatFit(area.clientHeight, roomy ? ROOMY_CHIP_HEIGHT : CHIP_HEIGHT))
      }
    }
    // Observing reports the size at once, so a view changing to the week
    // measures again with the week's chip, without a resize to wait for.
    const observer = new ResizeObserver(measure)
    observer.observe(cells)
    return () => observer.disconnect()
  }, [roomy])

  // The one day showing everything it holds, if any. One at a time: several
  // open days at once and the grid stops being a month at a glance, which is
  // the only reason the days fold in the first place.
  const [expanded, setExpanded] = useState<string | null>(null)

  // The row under the pointer, for drawing the carried chip.
  const carried = slots.find((slot) => slot.id === dragging?.id)

  const onDays = byDate(slots, (slot) => slot.scheduled_at)
  const now = today()
  const claiming = claimingId !== null

  // What the day under the pointer already holds. It was the dry run of a
  // contest until v0.44 and predicted a refusal; nothing is refused now, so it
  // says what is there and the drop happens either way. Since v0.90 it also
  // says what the release would repeat there (ADR 0054), and since v0.96
  // whether the day is closed - each a warning before it lands, never a
  // refusal (`useLandingVerdict`).
  const moving = dragging?.id ?? claimingId
  const onDay = over !== null && days.some((day) => day.date === over)
  const verdictFor = useLandingVerdict({
    moving,
    over: onDay ? over : null,
    slots: booked,
    closed,
  })

  return (
    <div
      ref={gridRef}
      className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden rounded-lg border border-line bg-raise"
    >
      <div className="grid shrink-0 grid-cols-7 border-b border-line">
        {WEEKDAYS.map((day) => (
          <div key={day} className="px-2 py-1.5 caption">
            {t(`calendar.weekday.${day}`)}
          </div>
        ))}
      </div>

      <div ref={cellsRef} className="grid min-h-0 flex-1 auto-rows-fr grid-cols-7">
        {days.map((day, index) => (
          <DayCell
            key={day.date}
            day={day}
            releases={onDays.get(day.date) ?? []}
            ghosts={ghosts?.get(day.date) ?? []}
            now={now}
            lines={lines}
            roomy={roomy}
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
            over={over === day.date}
            target={claiming || dragging !== null ? target(day.date) : undefined}
            verdict={verdictFor(day.date)}
            lastColumn={index % 7 === 6}
            lastRow={index >= days.length - 7}
          />
        ))}
      </div>

      {dragging !== null && carried !== undefined && (
        <CarriedChip dragging={dragging} slot={carried} now={now} roomy={roomy} />
      )}
    </div>
  )
}
