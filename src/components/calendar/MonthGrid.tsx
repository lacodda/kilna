import { useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { ChevronLeft, ChevronRight, Trash2 } from 'lucide-react'
import { previewSchedule, type ScheduledRelease } from '@/lib/api'
import type { Ghost } from '@/lib/layout'
import { byDate, monthGrid, sameMonth, shiftMonth, today, type Month } from '@/lib/month'
import { accentFor } from '@/lib/cover'
import { releaseIcon } from '@/lib/releaseIcon'
import { labelOf, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { SlotChip } from '@/components/calendar/SlotChip'
import { useChipDrag } from '@/lib/useChipDrag'
import { cn } from '@/lib/utils'

interface Props {
  month: Month
  onMonthChange: (month: Month) => void
  slots: readonly ScheduledRelease[]
  /** An auto-layout preview by day: where the queue would land, booked by
      nothing yet. Drawn as dashed chips among the real ones. */
  ghosts?: Map<string, Ghost[]>
  /** The queued release waiting for a date, if any; clicking a day takes it. */
  claimingId: string | null
  onPickDay: (date: string) => void
  onOpenRelease: (releaseId: string) => void
  /** A release dropped on a different day. */
  onMove: (releaseId: string, date: string) => void
  /** A release dragged onto the bin. */
  onUnschedule: (releaseId: string) => void
}

const WEEKDAYS = ['mon', 'tue', 'wed', 'thu', 'fri', 'sat', 'sun'] as const

/** How many chips a day shows before the rest fold into a count. */
const VISIBLE_CHIPS = 2

/**
 * A month at a time, with what is booked on each day.
 *
 * The flat list this replaces answered "what is scheduled" but never "is that
 * week empty" — which is the question a release plan is actually read for. The
 * strip of months moves in both directions rather than stopping at the current
 * one: work is planned ahead and reviewed behind.
 */
export function MonthGrid({
  month,
  onMonthChange,
  slots,
  ghosts,
  claimingId,
  onPickDay,
  onOpenRelease,
  onMove,
  onUnschedule,
}: Props) {
  const { t, i18n } = useTranslation()
  const profile = useProfile()

  // Where the grid is on screen, so the hook knows when the pointer has
  // reached an edge and the month should turn.
  const gridRef = useRef<HTMLDivElement>(null)

  // The day under the pointer, or the bin. Set from the carried chip's
  // position rather than from the day's own hover, because the ghost sits
  // under the pointer and a day cannot see through it.
  const [over, setOver] = useState<string | null>(null)

  const { dragging, begin } = useChipDrag({
    gridRef,
    onEdge: (step) => onMonthChange(shiftMonth(month, step)),
    onDrop: (id, target) => {
      setOver(null)
      const day = target?.closest<HTMLElement>('[data-day]')?.dataset.day
      if (day !== undefined) {
        onMove(id, day)
        return
      }
      if (target?.closest('[data-bin]') != null) onUnschedule(id)
    },
  })

  // The profile's entry for a kind, or nothing when the calendar still holds a
  // release of a kind the profile has since dropped. Nothing is a real answer
  // here: the chip falls back to the neutral glyph rather than disappearing.
  const kindOf = (key: string) => profile.config.release_kinds.find((kind) => kind.key === key)

  // The one day showing everything it holds, if any. One at a time: several
  // expanded days at once and the grid stops being a month at a glance, which
  // is the only reason it is collapsed in the first place.
  const [expanded, setExpanded] = useState<string | null>(null)

  const shownOn = (date: string, releases: readonly ScheduledRelease[]) =>
    expanded === date ? releases : releases.slice(0, VISIBLE_CHIPS)

  // The row under the pointer, for drawing the ghost.
  const carried = slots.find((slot) => slot.id === dragging?.id)

  const days = monthGrid(month)
  const booked = byDate(slots, (slot) => slot.scheduled_at)
  const now = today()
  const claiming = claimingId !== null

  // What the day under the pointer already holds. It was the dry run of a
  // contest until v0.44 and predicted a refusal; nothing is refused now, so it
  // says what is there and the drop happens either way.
  const moving = dragging?.id ?? claimingId
  const preview = useQuery({
    queryKey: ['slotPreview', moving, over],
    queryFn: () => previewSchedule(moving as string, over as string),
    enabled: moving !== null && over !== null && over !== 'bin',
    staleTime: 5_000,
  })
  const verdictFor = (date: string) =>
    over === date && preview.data !== undefined && preview.data.verdict !== 'empty'
      ? preview.data
      : null

  const title = new Date(month.year, month.month - 1, 1).toLocaleDateString(i18n.language, {
    month: 'long',
    year: 'numeric',
  })

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center gap-2">
        <Button
          variant="icon"
          size="icon-sm"
          aria-label={t('calendar.previousMonth')}
          title={t('calendar.previousMonth')}
          onClick={() => onMonthChange(shiftMonth(month, -1))}
        >
          <ChevronLeft aria-hidden />
        </Button>
        <h3 className="min-w-44 text-center text-sm font-semibold capitalize">{title}</h3>
        <Button
          variant="icon"
          size="icon-sm"
          aria-label={t('calendar.nextMonth')}
          title={t('calendar.nextMonth')}
          onClick={() => onMonthChange(shiftMonth(month, 1))}
        >
          <ChevronRight aria-hidden />
        </Button>

        {/* Only while something is in the air. A bin standing there permanently
            invites the question "what does this delete?"; appearing under a
            dragged release, it can only mean one thing. */}
        {dragging !== null && (
          <div
            data-bin
            onPointerEnter={() => setOver('bin')}
            onPointerLeave={() => setOver((current) => (current === 'bin' ? null : current))}
            className={cn(
              'ml-auto flex items-center gap-1.5 rounded-[10px] border border-dashed px-3 py-1 text-xs transition-colors',
              over === 'bin'
                ? 'border-bad bg-bad-soft text-bad'
                : 'border-line-2 text-dim',
            )}
          >
            <Trash2 aria-hidden className="size-3.5" />
            {t('calendar.dropToUnschedule')}
          </div>
        )}

        {dragging === null &&
          !sameMonth(month, { year: Number(now.slice(0, 4)), month: Number(now.slice(5, 7)) }) && (
          <Button
            size="sm"
            className="ml-2"
            onClick={() =>
              onMonthChange({ year: Number(now.slice(0, 4)), month: Number(now.slice(5, 7)) })
            }
          >
            {t('calendar.thisMonth')}
          </Button>
        )}
      </div>

      <div
        ref={gridRef}
        className="grid grid-cols-7 gap-px overflow-hidden rounded-xl border border-line bg-line"
      >
        {WEEKDAYS.map((day) => (
          <div
            key={day}
            className="bg-raise py-1.5 text-center text-[10px] font-semibold uppercase tracking-[0.08em] text-faint"
          >
            {t(`calendar.weekday.${day}`)}
          </div>
        ))}

        {days.map((day) => {
          const releases = booked.get(day.date) ?? []
          const isToday = day.date === now
          const verdict = verdictFor(day.date)

          return (
            <div
              key={day.date}
              // The date this cell stands for, read back from the element the
              // pointer was released over. A day of a neighbouring month is
              // dimmed but no longer inert: dragging reaches it, and the month
              // turns under the pointer anyway.
              data-day={day.date}
              className={cn(
                'min-h-24 bg-bg p-1.5 transition-colors',
                !day.inMonth && 'opacity-40',
                // Today is where the eye starts. The number alone carried it
                // until now, and on a grid of forty-two cells a coloured digit
                // is not where the eye starts.
                isToday && 'inset-ring inset-ring-accent',
                claiming && day.inMonth && 'cursor-pointer hover:bg-soft',
                // Somewhere to land. No red: nothing is refused any more, and
                // a day that already holds something says so in words below.
                over === day.date && 'bg-accent-soft',
              )}
              onClick={claiming && day.inMonth ? () => onPickDay(day.date) : undefined}
              // One set of handlers for both gestures: the queue's
              // click-to-book and a chip in the air. The carried ghost is
              // `pointer-events: none`, so the day underneath keeps receiving
              // the pointer and lights up as it is crossed.
              onPointerEnter={
                claiming || dragging !== null ? () => setOver(day.date) : undefined
              }
              onPointerLeave={
                claiming || dragging !== null
                  ? () => setOver((current) => (current === day.date ? null : current))
                  : undefined
              }
            >
              <div
                className={cn(
                  'mb-1 text-right font-mono text-[11px] tabular-nums',
                  isToday ? 'font-semibold text-accent-2' : 'text-faint',
                )}
              >
                {Number(day.date.slice(8))}
              </div>

              <div className="flex flex-col gap-1">
                {shownOn(day.date, releases).map((slot) => (
                  <SlotChip
                    key={slot.id}
                    slot={slot}
                    date={day.date}
                    now={now}
                    dragging={dragging?.id === slot.id}
                    onGrab={(event) => begin(event, slot.id)}
                    onOpen={() => onOpenRelease(slot.id)}
                  />
                ))}

                {/* A day holds as many releases as are put on it since v0.44,
                    and a cell ~130px wide holds two before the week's rows
                    start drifting apart. The rest are one line away rather
                    than hidden: the count is the whole point. */}
                {releases.length > VISIBLE_CHIPS && expanded !== day.date && (
                  <button
                    type="button"
                    onClick={(event) => {
                      event.stopPropagation()
                      setExpanded(day.date)
                    }}
                    className="cursor-pointer rounded-[7px] px-1 py-0.5 text-left text-[10px] text-dim transition-colors hover:bg-soft hover:text-text"
                  >
                    {t('calendar.moreOnDay', { count: releases.length - VISIBLE_CHIPS })}
                  </button>
                )}
                {expanded === day.date && releases.length > VISIBLE_CHIPS && (
                  <button
                    type="button"
                    onClick={(event) => {
                      event.stopPropagation()
                      setExpanded(null)
                    }}
                    className="cursor-pointer rounded-[7px] px-1 py-0.5 text-left text-[10px] text-dim transition-colors hover:bg-soft hover:text-text"
                  >
                    {t('calendar.showFewer')}
                  </button>
                )}

                {/* Where the auto-layout would put things: dashed and in the
                    work's own colour as an outline, so a plan is visibly not
                    a booking. Inert on purpose — the plan is approved or
                    cancelled from the bar above, not edited chip by chip. */}
                {(ghosts?.get(day.date) ?? []).map((ghost) => (
                  <div
                    key={ghost.releaseId}
                    title={`${ghost.title} · ${labelOf(profile.config.release_kinds, ghost.kind)}`}
                    className="flex flex-wrap items-center gap-x-1.5 rounded-[7px] border border-dashed px-1.5 py-1 text-[11px]"
                    style={{ borderColor: accentFor(ghost.workId) }}
                  >
                    {/* Same two rows as a booked chip, for the same reason: on
                        a ~100px day the kind spelled out ate the title. */}
                    {(() => {
                      const Glyph = releaseIcon(kindOf(ghost.kind)?.icon)
                      return <Glyph aria-hidden className="size-3 shrink-0 text-faint" />
                    })()}
                    <span className="w-full truncate font-medium leading-tight text-dim">
                      {ghost.title}
                    </span>
                  </div>
                ))}

                {verdict !== null && (
                  <p
                    className={cn(
                      'rounded-[6px] px-1 py-0.5 text-[10px] leading-tight',
                      verdict.verdict === 'displaces'
                        ? 'bg-warn-soft text-warn'
                        : 'bg-bad-soft text-bad',
                    )}
                  >
                    {t(`calendar.preview.${verdict.verdict}`, {
                      title: verdict.holder_title ?? '',
                    })}
                  </p>
                )}
              </div>
            </div>
          )
        })}
      </div>

      {/* The chip that follows the pointer. A real one, drawn by React rather
          than a bitmap the browser snapshots: it keeps the work's colour, its
          marks and its title, and it can be styled while it travels. Fixed to
          the viewport, so no ancestor's overflow can clip it. */}
      {dragging !== null &&
        carried !== undefined &&
        createPortal(
          <div
            className="pointer-events-none fixed z-50"
            style={{
              left: dragging.ghost.left,
              top: dragging.ghost.top,
              width: dragging.size.width,
            }}
          >
            <SlotChip
              slot={carried}
              date={carried.scheduled_at ?? now}
              now={now}
              dragging={false}
              onGrab={() => {}}
              onOpen={() => {}}
              asGhost
            />
          </div>,
          document.body,
        )}
    </div>
  )
}
