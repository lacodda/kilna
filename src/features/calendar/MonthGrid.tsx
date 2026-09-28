import { useEffect, useRef, useState, type RefObject } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { ScheduledRelease } from '@/lib/api/types'
import type { Ghost } from '@/lib/layout'
import { byDate, monthGrid, today, type Month } from '@/lib/month'
import { linesThatFit, UNMEASURED_LINES } from '@/lib/calendarDay'
import { queries } from '@/lib/query/queries'
import type { Dragging } from '@/lib/useChipDrag'
import { CarriedChip } from '@/features/calendar/CarriedChip'
import { DayCell } from '@/features/calendar/DayCell'
import type { DropTarget } from '@/features/calendar/useCalendarDrag'

interface Props {
  month: Month
  slots: readonly ScheduledRelease[]
  /** An auto-layout preview by day: where the queue would land, booked by
      nothing yet. Drawn as dashed chips among the real ones. */
  ghosts?: Map<string, Ghost[]>
  /** The queued release waiting for a date, if any; clicking a day takes it. */
  claimingId: string | null
  onPickDay: (date: string) => void
  onOpenRelease: (releaseId: string) => void
  /** Add a work that goes out on this day: the `+` a day shows under the
   * pointer, and the one entry in its right-click menu. */
  onAddOn: (date: string) => void
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
 * A month at a time, with what is booked on each day.
 *
 * The flat list this replaced answered "what is scheduled" but never "is that
 * week empty" - which is the question a release plan is actually read for.
 *
 * The month holds the window's height: its weeks share it (`auto-rows-fr`)
 * and a day never grows past its share, so the grid does not scroll and the
 * screen under it cannot. What a day has no room for folds into a count
 * (`DayCell`). Its title and arrows are the screen's head since v0.79, one row
 * with the filters and the actions, as the mockup draws it.
 */
export function MonthGrid({
  month,
  slots,
  ghosts,
  claimingId,
  onPickDay,
  onOpenRelease,
  onAddOn,
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
      if (area !== null) setLines(linesThatFit(area.clientHeight))
    }
    const observer = new ResizeObserver(measure)
    observer.observe(cells)
    return () => observer.disconnect()
  }, [])

  // The one day showing everything it holds, if any. One at a time: several
  // open days at once and the grid stops being a month at a glance, which is
  // the only reason the days fold in the first place.
  const [expanded, setExpanded] = useState<string | null>(null)

  // The row under the pointer, for drawing the carried chip.
  const carried = slots.find((slot) => slot.id === dragging?.id)

  const days = monthGrid(month)
  const booked = byDate(slots, (slot) => slot.scheduled_at)
  const now = today()
  const claiming = claimingId !== null

  // What the day under the pointer already holds. It was the dry run of a
  // contest until v0.44 and predicted a refusal; nothing is refused now, so it
  // says what is there and the drop happens either way.
  const moving = dragging?.id ?? claimingId
  const onDay = over !== null && days.some((day) => day.date === over)
  const preview = useQuery({
    ...queries.slotPreview(moving as string, over as string),
    enabled: moving !== null && onDay,
    staleTime: 5_000,
  })
  const verdictFor = (date: string) =>
    over === date && preview.data !== undefined && preview.data.verdict !== 'empty'
      ? preview.data
      : null

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
            releases={booked.get(day.date) ?? []}
            ghosts={ghosts?.get(day.date) ?? []}
            now={now}
            lines={lines}
            expanded={expanded === day.date}
            onExpand={(open) => setExpanded(open ? day.date : null)}
            claiming={claiming}
            onPick={() => onPickDay(day.date)}
            onAdd={() => onAddOn(day.date)}
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
        <CarriedChip dragging={dragging} slot={carried} now={now} />
      )}
    </div>
  )
}
