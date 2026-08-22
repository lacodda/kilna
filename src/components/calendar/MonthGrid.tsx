import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ChevronLeft, ChevronRight, GripVertical, Lock, Trash2 } from 'lucide-react'
import type { ScheduledRelease } from '@/lib/api'
import { byDate, monthGrid, sameMonth, shiftMonth, today, type Month } from '@/lib/month'
import { coverFor } from '@/lib/cover'
import { labelOf, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/Button'
import { cn } from '@/lib/utils'

interface Props {
  month: Month
  onMonthChange: (month: Month) => void
  slots: readonly ScheduledRelease[]
  /** Set while a queued release is waiting for a date; clicking a day takes it. */
  claiming: boolean
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
  claiming,
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

  const title = new Date(month.year, month.month - 1, 1).toLocaleDateString(i18n.language, {
    month: 'long',
    year: 'numeric',
  })

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center gap-2">
        <Button
          variant="icon"
          size="iconSm"
          aria-label={t('calendar.previousMonth')}
          title={t('calendar.previousMonth')}
          onClick={() => onMonthChange(shiftMonth(month, -1))}
        >
          <ChevronLeft aria-hidden />
        </Button>
        <h3 className="min-w-44 text-center text-sm font-semibold capitalize">{title}</h3>
        <Button
          variant="icon"
          size="iconSm"
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

          return (
            <div
              key={day.date}
              // A day of a neighbouring month is drawn, but dimmed and inert:
              // clicking it would book a date you cannot see the rest of.
              className={cn(
                'min-h-24 bg-bg p-1.5 transition-colors',
                !day.inMonth && 'opacity-40',
                claiming && day.inMonth && 'cursor-pointer hover:bg-soft',
                over === day.date && 'bg-accent-soft',
              )}
              onClick={claiming && day.inMonth ? () => onPickDay(day.date) : undefined}
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
                      'flex items-center gap-1 rounded-[7px] px-1 py-1 text-[11px] transition-opacity',
                      slot.status === 'released' && 'opacity-60',
                      dragging === slot.id && 'opacity-40',
                    )}
                    // The work's own colour, so the same song is the same colour
                    // wherever it appears — the cover, the card, this chip.
                    style={{ background: coverFor(slot.work_id) }}
                  >
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
                      className="flex min-w-0 flex-1 cursor-pointer items-center gap-1.5 text-left hover:opacity-80"
                    >
                      <span className="min-w-0 flex-1 truncate font-medium text-white/90">
                        {slot.work_title}
                      </span>
                      {slot.slot_pinned_at !== null && (
                        <Lock
                          aria-hidden
                          className="size-2.5 shrink-0 text-white/70"
                        />
                      )}
                      <span className="shrink-0 text-[9px] uppercase tracking-wide text-white/60">
                        {labelOf(profile.config.release_kinds, slot.kind)}
                      </span>
                    </button>
                  </div>
                ))}
              </div>
            </div>
          )
        })}
      </div>
    </div>
  )
}
