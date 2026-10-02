import { useTranslation } from 'react-i18next'
import { Plus } from 'lucide-react'
import type { ScheduledRelease, SlotPreview, SlotVerdict } from '@/lib/api/types'
import type { Ghost } from '@/lib/layout'
import type { Day } from '@/lib/month'
import { foldDay } from '@/lib/calendarDay'
import { formatDay } from '@/lib/format'
import { landingWarning } from '@/lib/repeats'
import { RowContextMenu } from '@/components/RowMenu'
import { Button } from '@/components/ui/button'
import { Scroll } from '@/components/frame'
import { GhostChip } from '@/features/calendar/GhostChip'
import { SlotChip } from '@/features/calendar/SlotChip'
import type { DropTarget } from '@/features/calendar/useCalendarDrag'
import { cn } from '@/lib/utils'

interface Props {
  day: Day
  releases: readonly ScheduledRelease[]
  /** Where the auto-layout would land queued releases on this day. */
  ghosts: readonly Ghost[]
  now: string
  /** How many chip lines the day has room for (`lib/calendarDay`). */
  lines: number
  /** Showing everything it holds, rolled inside the day, rather than folded. */
  expanded: boolean
  onExpand: (expanded: boolean) => void
  /** A queued release is waiting for a date: pressing the day gives it this one. */
  claiming: boolean
  onPick: () => void
  /** Add a work that goes out on this day. */
  onAdd: () => void
  onOpenRelease: (releaseId: string) => void
  /** The release in the air, if any. */
  carrying: string | null
  onGrab: (event: React.PointerEvent, releaseId: string) => void
  /** Whether something is over this day that could land on it. */
  over: boolean
  /** The handlers that light the day up under a chip or a pick, when either is live. */
  target?: DropTarget
  /** What the day already holds, and what the release would repeat on it,
   *  said while something is over it. */
  verdict: SlotPreview | null
  /** Where the day sits in the grid, for which of its hairlines to draw. */
  lastColumn: boolean
  lastRow: boolean
}

/**
 * One day of the month, as tall as the window makes it.
 *
 * The day never grows: it shows as many one-line chips as fit and folds the
 * rest into a count. The count opens the day in place - the same height, its
 * chips rolled inside it - so looking at a busy Friday does not push the
 * following week off the bottom of the window.
 */
export function DayCell({
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
  onOpenRelease,
  carrying,
  onGrab,
  over,
  target,
  verdict,
  lastColumn,
  lastRow,
}: Props) {
  const { t } = useTranslation()
  const isToday = day.date === now
  const repeats = verdict === null ? null : landingWarning(verdict.repeats)

  // Booked chips first, then the plan's: a booking is a fact, a ghost is the
  // question the bar above the month is asking.
  const chips = [
    ...releases.map((slot) => (
      <SlotChip
        key={slot.id}
        slot={slot}
        date={day.date}
        now={now}
        dragging={carrying === slot.id}
        onGrab={(event) => onGrab(event, slot.id)}
        onOpen={() => onOpenRelease(slot.id)}
      />
    )),
    ...ghosts.map((ghost) => <GhostChip key={ghost.releaseId} ghost={ghost} />),
  ]

  const folded = foldDay(chips.length, lines)
  // Open only while there is something folded away: a window made taller
  // with a day open shows everything anyway, and a "show fewer" under it
  // would fold nothing.
  const open = expanded && folded.more > 0

  return (
    <RowContextMenu
      actions={[{ key: 'add', label: t('calendar.addOnDay'), onSelect: onAdd }]}
      render={
        <div
          // The date this cell stands for, read back from the element the
          // pointer was released over. A day of a neighbouring month is on a
          // quieter ground but not inert: dragging reaches it, and the month
          // turns under the pointer anyway.
          data-day={day.date}
          className={cn(
            'group relative flex min-h-0 min-w-0 flex-col gap-0.5 overflow-hidden border-line p-1 transition-colors',
            !lastColumn && 'border-r',
            !lastRow && 'border-b',
            !day.inMonth && 'bg-softer',
            claiming && day.inMonth && 'cursor-pointer hover:bg-soft',
            // Somewhere to land. No red: nothing is refused any more, and a
            // day that already holds something says so in words below.
            over && 'bg-accent-soft',
          )}
          onClick={claiming && day.inMonth ? onPick : undefined}
          // One set of handlers for both gestures: the queue's click-to-book
          // and a chip in the air. The carried chip is `pointer-events: none`,
          // so the day underneath keeps receiving the pointer and lights up
          // as it is crossed.
          onPointerEnter={target?.onPointerEnter}
          onPointerLeave={target?.onPointerLeave}
        />
      }
    >
      <div className="flex h-5 shrink-0 items-center justify-between gap-1">
        {/* Today is where the eye starts, so it is filled rather than
            coloured: on a grid of forty-two cells a coloured digit is not
            where the eye starts. */}
        <span
          className={cn(
            'rounded-xs px-1 font-mono text-2xs tabular-nums',
            isToday ? 'bg-accent font-semibold text-on-accent' : 'text-faint',
          )}
        >
          {Number(day.date.slice(8))}
        </span>

        {/* Under the pointer only: a plus on every one of forty-two cells is
            forty-two plus signs to read past. It is `opacity-0` rather than
            absent so the row does not reflow as the pointer crosses the
            month, and `focus-visible` brings it back for the keyboard, which
            has no hover to offer. */}
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
          className={cn('min-h-0', !day.inMonth && 'opacity-60')}
          contentClassName="flex flex-col gap-0.5"
        >
          {chips}
          <FoldButton onClick={() => onExpand(false)}>{t('calendar.showFewer')}</FoldButton>
        </Scroll>
      ) : (
        <div
          // What `MonthGrid` measures: the part of a day its chips have,
          // the same in every day of the month.
          data-day-lines
          className={cn(
            'flex min-h-0 flex-1 flex-col gap-0.5 overflow-hidden',
            !day.inMonth && 'opacity-60',
          )}
        >
          {chips.slice(0, folded.shown)}
          {/* The rest are one line away rather than hidden: the count is the
              whole point. */}
          {folded.more > 0 && (
            <FoldButton onClick={() => onExpand(true)}>
              {t('calendar.moreOnDay', { count: folded.more })}
            </FoldButton>
          )}
        </div>
      )}

      {/* Laid over the foot of the day rather than under its chips: the day
          does not grow, and the verdict is only up while something hovers.
          Who is there, and what the release would repeat there (ADR 0054),
          a line each - the repeat last, nearest the foot, in the guard's
          red: it is the one that may change where the release goes. */}
      {verdict !== null && (
        <div className="pointer-events-none absolute inset-x-1 bottom-1 flex flex-col gap-0.5">
          {verdict.verdict !== 'empty' && (
            <p
              className={cn(
                'rounded-sm px-1 py-0.5 text-2xs leading-tight shadow-lift',
                VERDICT_TONE[verdict.verdict],
              )}
            >
              {t(`calendar.preview.${verdict.verdict}`, { title: verdict.holder_title ?? '' })}
            </p>
          )}
          {repeats !== null && (
            <p
              role="status"
              className="rounded-sm bg-raise px-1 py-0.5 text-2xs leading-tight text-bad shadow-lift"
            >
              {repeats}
            </p>
          )}
        </div>
      )}
    </RowContextMenu>
  )
}

/** The line that folds a day or opens it: a chip's height, so the sum holds. */
function FoldButton({ onClick, children }: { onClick: () => void; children: string }) {
  return (
    <Button
      variant="link"
      onClick={(event) => {
        event.stopPropagation()
        onClick()
      }}
      className="h-5 shrink-0 self-start px-1 text-xs"
    >
      {children}
    </Button>
  )
}

/**
 * How a day's verdict reads while something is dragged over it.
 *
 * Neither is a refusal - a day holds as many releases as are put on it - so
 * neither is red. Something already there is information; a date settled by
 * hand is worth a warmer look before adding beside it. Written as a record
 * over the verdicts, so a verdict the backend adds is a type error here until
 * it has a tone. Opaque grounds, since the line is laid over the day's chips.
 */
const VERDICT_TONE: Record<SlotVerdict, string> = {
  // Never drawn: an empty day says nothing while something is dragged over it.
  empty: '',
  taken: 'bg-raise text-dim',
  pinned: 'bg-raise text-warn',
}
