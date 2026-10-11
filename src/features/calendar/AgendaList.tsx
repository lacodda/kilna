import { useTranslation } from 'react-i18next'
import { CalendarOff, Plus } from 'lucide-react'
import type { ScheduledRelease, TimeOff } from '@/lib/api/types'
import { agendaRuns, type AgendaRun } from '@/lib/agenda'
import { formatDay, formatWeekday } from '@/lib/format'
import type { Ghost } from '@/lib/layout'
import { byDate, today, type Day } from '@/lib/month'
import { landingWarning } from '@/lib/repeats'
import { spanText } from '@/lib/timeOff'
import type { Dragging } from '@/lib/useChipDrag'
import { RowContextMenu, type RowAction } from '@/components/RowMenu'
import { Scroll } from '@/components/frame'
import { Button } from '@/components/ui/button'
import { CarriedChip } from '@/features/calendar/CarriedChip'
import { GhostChip } from '@/features/calendar/GhostChip'
import { SlotChip } from '@/features/calendar/SlotChip'
import type { DropTarget } from '@/features/calendar/useCalendarDrag'
import { useLandingVerdict } from '@/features/calendar/useLandingVerdict'
import { cn } from '@/lib/utils'

interface Props {
  /** The month's own days, in order. */
  days: readonly Day[]
  slots: readonly ScheduledRelease[]
  /** Everything booked, whatever the kind filter shows. */
  booked: readonly ScheduledRelease[]
  ghosts?: Map<string, Ghost[]>
  closed: ReadonlyMap<string, TimeOff>
  claimingId: string | null
  onPickDay: (date: string) => void
  onOpenRelease: (releaseId: string) => void
  onAddOn: (date: string) => void
  onTimeOff: (date: string, closedBy: TimeOff | null) => void
  onShiftFrom: (date: string) => void
  dragging: Dragging | null
  onGrab: (event: React.PointerEvent, releaseId: string) => void
  over: string | null
  target: (key: string) => DropTarget
}

/**
 * The month as a list (v0.96): each day with something on it under its own
 * heading, every release on it in full, and the days between folded into one
 * line that says how many - or, when they are closed, why.
 *
 * Read top to bottom where the grid is read at a glance. A day's heading is
 * where a release lands, by a click while one is picked or by letting go of a
 * carried one; the folded lines are not - a drop on "4 days with nothing
 * planned" would have to guess which of the four was meant.
 */
export function AgendaList({
  days,
  slots,
  booked,
  ghosts,
  closed,
  claimingId,
  onPickDay,
  onOpenRelease,
  onAddOn,
  onTimeOff,
  onShiftFrom,
  dragging,
  onGrab,
  over,
  target,
}: Props) {
  const { t } = useTranslation()
  const now = today()
  const onDays = byDate(slots, (slot) => slot.scheduled_at)
  const claiming = claimingId !== null
  const carrying = dragging !== null
  const carried = slots.find((slot) => slot.id === dragging?.id)

  const moving = dragging?.id ?? claimingId
  const onDay = over !== null && days.some((day) => day.date === over)
  const verdictFor = useLandingVerdict({
    moving,
    over: onDay ? over : null,
    slots: booked,
    closed,
  })

  const runs = agendaRuns(
    days.map((day) => day.date),
    (date) => (onDays.get(date)?.length ?? 0) > 0 || (ghosts?.get(date)?.length ?? 0) > 0,
    closed,
    now,
  )

  const actionsFor = (date: string, span: TimeOff | null): RowAction[] => [
    { key: 'add', label: t('calendar.addOnDay'), onSelect: () => onAddOn(date) },
    {
      key: 'timeOff',
      label: span === null ? t('timeOff.closeDay') : t('timeOff.editNamed', { title: span.label }),
      onSelect: () => onTimeOff(date, span),
    },
    { key: 'shift', label: t('shift.fromDay'), onSelect: () => onShiftFrom(date) },
  ]

  const dayGroup = (run: Extract<AgendaRun, { kind: 'day' }>) => {
    const verdict = verdictFor(run.date)
    const repeats = verdict === null ? null : landingWarning(verdict.repeats)
    const releases = onDays.get(run.date) ?? []
    const planned = ghosts?.get(run.date) ?? []
    const isToday = run.date === now

    return (
      <RowContextMenu
        key={run.date}
        actions={actionsFor(run.date, run.closed)}
        render={
          <section
            // Where a carried release lands, read back on release.
            data-day={run.date}
            aria-label={formatWeekday(run.date)}
            onClick={claiming ? () => onPickDay(run.date) : undefined}
            onPointerEnter={claiming || carrying ? target(run.date).onPointerEnter : undefined}
            onPointerLeave={claiming || carrying ? target(run.date).onPointerLeave : undefined}
            className={cn(
              'group flex flex-col gap-1 rounded-md border border-line px-2 py-1.5 transition-colors',
              claiming && 'cursor-pointer hover:bg-soft',
              over === run.date && 'bg-accent-soft',
            )}
          />
        }
      >
        <header className="flex items-center gap-2">
          <span
            className={cn(
              'rounded-xs px-1 text-xs font-semibold tabular-nums',
              isToday ? 'bg-accent text-on-accent' : 'text-dim',
            )}
          >
            {formatWeekday(run.date)}
          </span>
          {isToday && <span className="text-2xs text-faint">{t('calendar.today')}</span>}
          {run.closed !== null && (
            <span className="flex min-w-0 items-center gap-1 text-xs text-faint">
              <CalendarOff aria-hidden className="size-3.5 shrink-0" />
              <span className="truncate">{run.closed.label}</span>
            </span>
          )}
          <Button
            variant="icon"
            size="icon-xs"
            title={t('calendar.addOnDay')}
            aria-label={t('calendar.addOnDay')}
            onClick={(event) => {
              event.stopPropagation()
              onAddOn(run.date)
            }}
            className="ml-auto opacity-0 group-hover:opacity-100 focus-visible:opacity-100"
          >
            <Plus aria-hidden />
          </Button>
        </header>

        {(releases.length > 0 || planned.length > 0) && (
          <div className="flex flex-col gap-0.5">
            {releases.map((slot) => (
              <SlotChip
                key={slot.id}
                slot={slot}
                date={run.date}
                now={now}
                dragging={dragging?.id === slot.id}
                onGrab={(event) => onGrab(event, slot.id)}
                onOpen={() => onOpenRelease(slot.id)}
              />
            ))}
            {planned.map((ghost) => (
              <GhostChip key={ghost.releaseId} ghost={ghost} />
            ))}
          </div>
        )}

        {verdict !== null && (
          <div className="flex flex-col gap-0.5 text-2xs leading-tight">
            {verdict.closed !== null && (
              <p className="text-warn">{t('timeOff.landsOnClosed', { title: verdict.closed })}</p>
            )}
            {verdict.verdict !== 'empty' && (
              <p className={verdict.verdict === 'pinned' ? 'text-warn' : 'text-dim'}>
                {t(`calendar.preview.${verdict.verdict}`, { title: verdict.holder_title ?? '' })}
              </p>
            )}
            {repeats !== null && (
              <p role="status" className="text-bad">
                {repeats}
              </p>
            )}
          </div>
        )}
      </RowContextMenu>
    )
  }

  const gapLine = (run: Extract<AgendaRun, { kind: 'gap' }>) => {
    const when = spanText({ starts_on: run.from, ends_on: run.to }, (day) => formatDay(day))
    return (
      <RowContextMenu
        key={run.from}
        actions={actionsFor(run.from, run.closed)}
        render={
          <div
            className={cn(
              'flex items-center gap-2 px-2 py-0.5 text-xs text-faint',
              run.closed !== null && 'rounded-md bg-softer',
            )}
          />
        }
      >
        {run.closed !== null && <CalendarOff aria-hidden className="size-3.5 shrink-0" />}
        <span className="truncate">
          {run.closed !== null
            ? t('agenda.closed', { title: run.closed.label, when })
            : t('agenda.gap', { count: run.days, when })}
        </span>
      </RowContextMenu>
    )
  }

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden rounded-lg border border-line bg-raise">
      <Scroll
        label={t('agenda.label')}
        className="min-h-0 flex-1"
        contentClassName="flex flex-col gap-1.5 p-2"
      >
        {runs.map((run) => (run.kind === 'day' ? dayGroup(run) : gapLine(run)))}
      </Scroll>
      {dragging !== null && carried !== undefined && (
        <CarriedChip dragging={dragging} slot={carried} now={now} />
      )}
    </div>
  )
}
