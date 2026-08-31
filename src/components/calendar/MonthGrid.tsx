import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { ChevronLeft, ChevronRight, GripVertical, Lock, Trash2 } from 'lucide-react'
import { previewSchedule, type ScheduledRelease } from '@/lib/api'
import type { Ghost } from '@/lib/layout'
import { byDate, monthGrid, sameMonth, shiftMonth, today, type Month } from '@/lib/month'
import { accentFor, coverFor } from '@/lib/cover'
import { shortKind } from '@/lib/kind'
import { daysBetween } from '@/lib/readiness'
import { labelOf, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/Button'
import { ReadyMarks } from '@/components/calendar/ReadyMarks'
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

  // The release being dragged, so the bin can appear only while one is in the
  // air and the day under the cursor can light up.
  const [dragging, setDragging] = useState<string | null>(null)
  const [over, setOver] = useState<string | null>(null)

  const days = monthGrid(month)
  const booked = byDate(slots, (slot) => slot.scheduled_at)
  const now = today()
  const claiming = claimingId !== null

  // The dry run of the drop or the click: the same verdict the backend would
  // act on, asked for the day under the cursor. Announcing displacement after
  // the drop was v0.24; here it stops being a surprise.
  const moving = dragging ?? claimingId
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
            onDragOver={(event) => {
              event.preventDefault()
              setOver('bin')
            }}
            onDragLeave={() => setOver((current) => (current === 'bin' ? null : current))}
            onDrop={(event) => {
              event.preventDefault()
              setOver(null)
              const releaseId = event.dataTransfer.getData('text/plain')
              if (releaseId !== '') onUnschedule(releaseId)
            }}
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

      <div className="grid grid-cols-7 gap-px overflow-hidden rounded-xl border border-line bg-line">
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
          const refused = verdict !== null && verdict.verdict !== 'displaces'

          return (
            <div
              key={day.date}
              // A day of a neighbouring month is drawn, but dimmed and inert:
              // clicking it would book a date you cannot see the rest of.
              className={cn(
                'min-h-24 bg-bg p-1.5 transition-colors',
                !day.inMonth && 'opacity-40',
                claiming && day.inMonth && 'cursor-pointer hover:bg-soft',
                // The ground answers before the drop does: a day that would
                // refuse reads red, anything else reads like a place to land.
                over === day.date && (refused ? 'bg-bad-soft' : 'bg-accent-soft'),
              )}
              onClick={claiming && day.inMonth ? () => onPickDay(day.date) : undefined}
              // While a queued release waits for a date, hovering asks the
              // same question dragging does — the verdict shows before the
              // click books anything.
              onMouseEnter={claiming && day.inMonth ? () => setOver(day.date) : undefined}
              onMouseLeave={
                claiming
                  ? () => setOver((current) => (current === day.date ? null : current))
                  : undefined
              }
              onDragOver={(event) => {
                if (dragging === null || !day.inMonth) return
                // Without this the browser refuses the drop, and the whole
                // gesture ends in the cursor snapping back with no explanation.
                event.preventDefault()
                setOver(day.date)
              }}
              onDragLeave={() => setOver((current) => (current === day.date ? null : current))}
              onDrop={(event) => {
                event.preventDefault()
                setOver(null)
                const releaseId = event.dataTransfer.getData('text/plain')
                if (releaseId !== '' && day.inMonth) onMove(releaseId, day.date)
              }}
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
                {releases.map((slot) => (
                  <div
                    key={slot.id}
                    className={cn(
                      // A column, not a row. A day is ~100px wide, and one row
                      // spent all of it on the handle, the marks and the kind:
                      // the pilot's calendar showed three letters of every
                      // title. The handle and the marks share the top line; the
                      // title gets the day's full width underneath.
                      'flex flex-col gap-0.5 rounded-[7px] px-1 py-1 text-[11px] transition-opacity',
                      slot.status === 'released' && 'opacity-60',
                      dragging === slot.id && 'opacity-40',
                    )}
                    // The work's own colour, so the same song is the same colour
                    // wherever it appears — the cover, the card, this chip.
                    style={{ background: coverFor(slot.work_id) }}
                  >
                    {/* The top line: what you grab and what the release's state
                        is. Both are small and fixed-width, so they cost the
                        title nothing. */}
                    <div className="flex items-center gap-1">
                    {/* The handle, and only the handle, is draggable. Making the
                        whole chip draggable puts every click in a race with a
                        drag — the predecessor did that and lost the click. */}
                    <span
                      draggable={slot.status !== 'released'}
                      onDragStart={(event) => {
                        event.dataTransfer.setData('text/plain', slot.id)
                        event.dataTransfer.effectAllowed = 'move'
                        setDragging(slot.id)
                      }}
                      onDragEnd={() => {
                        setDragging(null)
                        setOver(null)
                      }}
                      onClick={(event) => event.stopPropagation()}
                      title={t('calendar.dragHandle')}
                      className={cn(
                        'shrink-0 text-white/50',
                        slot.status === 'released' ? 'opacity-30' : 'cursor-grab hover:text-white/80',
                      )}
                    >
                      <GripVertical aria-hidden className="size-3" />
                    </span>

                    <button
                      type="button"
                      onClick={(event) => {
                        // The day underneath books a date; the chip opens what
                        // is already booked. One click means one of those.
                        event.stopPropagation()
                        onOpenRelease(slot.id)
                      }}
                      title={`${slot.work_title} · ${labelOf(profile.config.release_kinds, slot.kind)}`}
                      className="flex min-w-0 flex-1 cursor-pointer items-center justify-end gap-1.5 text-left hover:opacity-80"
                    >
                      <ReadyMarks
                        readiness={slot.readiness}
                        released={slot.status === 'released'}
                        daysLeft={daysBetween(now, day.date)}
                        // The chip's ground is the work's own colour; the dark
                        // pill keeps the amber and red legible on any of them.
                        className={cn(
                          'rounded-[4px] bg-black/35 px-0.5 py-px',
                          slot.status === 'released' && 'text-white/70',
                        )}
                      />
                      {slot.slot_pinned_at !== null && (
                        <Lock
                          aria-hidden
                          className="size-2.5 shrink-0 text-white/70"
                        />
                      )}
                      {/* The kind, short enough to leave the title readable.
                          It used to be spelled out, and on a day holding two
                          releases "SHORT" left the title one letter wide. The
                          word itself stays in the chip's tooltip, and the kind
                          cannot be an icon: its keys come from the profile,
                          which the code does not get to know (ADR 0001). */}
                      <span
                        className="shrink-0 text-[9px] font-semibold tracking-wide text-white/55"
                        aria-hidden
                      >
                        {shortKind(labelOf(profile.config.release_kinds, slot.kind))}
                      </span>
                    </button>
                    </div>

                    {/* The title, on its own line and clickable like the marks
                        above it: the whole chip opens the release. */}
                    <button
                      type="button"
                      onClick={(event) => {
                        event.stopPropagation()
                        onOpenRelease(slot.id)
                      }}
                      title={`${slot.work_title} · ${labelOf(profile.config.release_kinds, slot.kind)}`}
                      className="w-full cursor-pointer truncate text-left font-medium leading-tight text-white/90 hover:opacity-80"
                    >
                      {slot.work_title}
                    </button>
                  </div>
                ))}

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
                    <span className="shrink-0 text-[9px] font-semibold tracking-wide text-faint">
                      {shortKind(labelOf(profile.config.release_kinds, ghost.kind))}
                    </span>
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
    </div>
  )
}
