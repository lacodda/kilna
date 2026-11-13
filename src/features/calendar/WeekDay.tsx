import { useTranslation } from 'react-i18next'
import { CalendarOff, Plus } from 'lucide-react'
import type { ScheduledRelease, SlotPreview, TimeOff } from '@/lib/api/types'
import type { Ghost } from '@/lib/layout'
import type { Day } from '@/lib/month'
import { foldDay } from '@/components/ui/big-calendar'
import { formatDay } from '@/lib/format'
import { RowContextMenu } from '@/components/RowMenu'
import { Button } from '@/components/ui/button'
import { Scroll } from '@/components/Scroll'
import { dayActions } from '@/features/calendar/days'
import { LandingLines, landingSays } from '@/features/calendar/LandingLines'
import { PlannedLine, ReleaseLine } from '@/features/calendar/ReleaseLine'
import { cn } from '@/lib/utils'

interface Props {
  day: Day
  releases: readonly ScheduledRelease[]
  /** Where the auto-layout would land queued releases on this day. */
  ghosts: readonly Ghost[]
  now: string
  /** How many lines the day has room for (`linesThatFit`). */
  lines: number
  /** Showing everything it holds, rolled inside the day, rather than folded. */
  expanded: boolean
  onExpand: (expanded: boolean) => void
  /** A queued release is waiting for a date: pressing the day gives it this one. */
  claiming: boolean
  onPick: () => void
  /** Add a work that goes out on this day. */
  onAdd: () => void
  /** The time off that closes the day, if any (v0.96). */
  closed: TimeOff | null
  /** Close this day, or open the span that closes it to edit. */
  onTimeOff: () => void
  /** Shift everything planned from this day on. */
  onShiftFrom: () => void
  onOpenRelease: (releaseId: string) => void
  /** The release in the air, if any. */
  carrying: string | null
  onGrab: (event: React.PointerEvent, releaseId: string) => void
  /** Whether something is over this day that could land on it. */
  over: boolean
  /** What the day already holds, and what the release would repeat on it,
   *  said while something is over it. */
  verdict: SlotPreview | null
  /** Whether the day is the last of the row, which draws no hairline after it. */
  lastColumn: boolean
}

/**
 * One day of the week, as tall as the window makes it.
 *
 * The day never grows: it shows as many lines as fit and folds the rest into
 * a count. The count opens the day in place - the same height, its lines
 * rolled inside it - so a busy Friday does not push anything off the window.
 * Its lines are the month's (`ReleaseLine`), twenty-four pixels each and edge
 * to edge, so the sum `foldDay` makes is exact.
 */
export function WeekDay({
  day,
  releases,
  ghosts,
  now,
  lines,
  expanded,
  onExpand,
  claiming,
  onPick,
  onAdd,
  closed,
  onTimeOff,
  onShiftFrom,
  onOpenRelease,
  carrying,
  onGrab,
  over,
  verdict,
  lastColumn,
}: Props) {
  const { t } = useTranslation()
  const isToday = day.date === now

  // Booked lines first, then the plan's: a booking is a fact, a planned line
  // is the question the bar above the week is asking.
  const items = [
    ...releases.map((slot) => (
      <ReleaseLine
        key={slot.id}
        slot={slot}
        date={day.date}
        now={now}
        dragging={carrying === slot.id}
        onGrab={(event) => onGrab(event, slot.id)}
        onOpen={() => onOpenRelease(slot.id)}
      />
    )),
    ...ghosts.map((ghost) => <PlannedLine key={`plan-${ghost.releaseId}`} ghost={ghost} />),
  ]

  const folded = foldDay(items.length, lines)
  // Open only while there is something folded away: a window made taller
  // with a day open shows everything anyway, and a "show fewer" under it
  // would fold nothing.
  const open = expanded && folded.more > 0

  return (
    <RowContextMenu
      actions={dayActions(t, closed, { add: onAdd, timeOff: onTimeOff, shift: onShiftFrom })}
      render={
        <div
          // The date this cell stands for, read back from the element the
          // pointer is over or was released over (`dayOf`).
          data-day={day.date}
          className={cn(
            'group relative flex min-h-0 min-w-0 flex-col gap-1 overflow-hidden p-1 transition-colors',
            !lastColumn && 'border-r border-line',
            // A closed day is hatched: still a day - something can be put on
            // it by hand - but plainly not one the plan meant to fill.
            closed !== null && CLOSED_GROUND,
            claiming && 'cursor-pointer hover:bg-soft',
            // Somewhere to land. No red: nothing is refused any more, and a
            // day that already holds something says so in words below.
            over && 'bg-accent-soft',
          )}
          onClick={claiming ? onPick : undefined}
        />
      }
    >
      <div className="flex h-5 shrink-0 items-center justify-between gap-1">
        {/* Today is where the eye starts, so it is filled rather than
            coloured. */}
        <span className="flex min-w-0 items-center gap-1">
          <span
            className={cn(
              'shrink-0 rounded-xs px-1 font-mono text-2xs tabular-nums',
              isToday ? 'bg-accent font-semibold text-on-accent' : 'text-dim',
            )}
          >
            {Number(day.date.slice(8))}
          </span>
          {/* Why the day is closed, in the line the date already takes. */}
          {closed !== null && (
            <span
              title={t('timeOff.closedBy', { title: closed.label })}
              className="flex min-w-0 items-center gap-0.5 text-2xs text-faint"
            >
              <CalendarOff aria-hidden className="size-3 shrink-0" />
              <span className="truncate">{closed.label}</span>
            </span>
          )}
        </span>

        {/* Under the pointer only, and `opacity-0` rather than absent so the
            row does not reflow as the pointer crosses the week;
            `focus-visible` brings it back for the keyboard. */}
        <Button
          variant="icon"
          size="icon-xs"
          title={t('calendar.addOnDay')}
          aria-label={t('calendar.addOnDay')}
          onClick={(event) => {
            event.stopPropagation()
            onAdd()
          }}
          className="opacity-0 group-hover:opacity-100 focus-visible:opacity-100"
        >
          <Plus aria-hidden />
        </Button>
      </div>

      {open ? (
        <Scroll
          label={t('calendar.dayReleases', { date: formatDay(day.date) })}
          className="min-h-0"
          contentClassName="flex flex-col"
        >
          {items}
          <FoldButton onClick={() => onExpand(false)}>{t('calendar.showFewer')}</FoldButton>
        </Scroll>
      ) : (
        <div
          // What `WeekGrid` measures: the part of a day its lines have, the
          // same in every day of the week.
          data-day-lines
          className="flex min-h-0 flex-1 flex-col overflow-hidden"
        >
          {items.slice(0, folded.shown)}
          {/* The rest are one line away rather than hidden: the count is the
              whole point. */}
          {folded.more > 0 && (
            <FoldButton onClick={() => onExpand(true)}>
              {t('calendar.moreOnDay', { count: folded.more })}
            </FoldButton>
          )}
        </div>
      )}

      {/* Laid over the foot of the day rather than under its lines, as the
          month lays it: the day does not grow, and the verdict is only up
          while something hovers. */}
      {over && landingSays(verdict) && (
        <p className="pointer-events-none absolute inset-x-1 bottom-1 rounded-sm bg-raise px-1 py-0.5 text-2xs leading-tight text-dim shadow-lift">
          <LandingLines verdict={verdict} />
        </p>
      )}
    </RowContextMenu>
  )
}

/** The line that folds a day or opens it: a line's height, so the sum holds. */
function FoldButton({ onClick, children }: { onClick: () => void; children: string }) {
  return (
    <Button
      variant="link"
      onClick={(event) => {
        event.stopPropagation()
        onClick()
      }}
      className="h-6 shrink-0 self-start px-1 text-xs"
    >
      {children}
    </Button>
  )
}

/**
 * The ground of a closed day: a fine hatch in the hairline's colour, over
 * whatever ground the day already has.
 */
const CLOSED_GROUND =
  '[background-image:repeating-linear-gradient(135deg,var(--line-2)_0_1px,transparent_1px_7px)]'
