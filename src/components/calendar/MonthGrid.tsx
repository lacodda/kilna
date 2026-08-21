import { useTranslation } from 'react-i18next'
import { ChevronLeft, ChevronRight } from 'lucide-react'
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
}: Props) {
  const { t, i18n } = useTranslation()
  const profile = useProfile()

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

        {!sameMonth(month, { year: Number(now.slice(0, 4)), month: Number(now.slice(5, 7)) }) && (
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
              )}
              onClick={claiming && day.inMonth ? () => onPickDay(day.date) : undefined}
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
                  <button
                    key={slot.id}
                    type="button"
                    onClick={(event) => {
                      // The day underneath books a date; the chip opens what is
                      // already booked. One click has to mean one of those.
                      event.stopPropagation()
                      onOpenRelease(slot.id)
                    }}
                    title={`${slot.work_title} · ${labelOf(profile.config.release_kinds, slot.kind)}`}
                    className={cn(
                      'flex w-full cursor-pointer items-center gap-1.5 rounded-[7px] px-1.5 py-1 text-left text-[11px] transition-opacity hover:opacity-80',
                      slot.status === 'released' && 'opacity-60',
                    )}
                    // The work's own colour, so the same song is the same colour
                    // wherever it appears — the cover, the card, this chip.
                    style={{ background: coverFor(slot.work_id) }}
                  >
                    <span className="min-w-0 flex-1 truncate font-medium text-white/90">
                      {slot.work_title}
                    </span>
                    <span className="shrink-0 text-[9px] uppercase tracking-wide text-white/60">
                      {labelOf(profile.config.release_kinds, slot.kind)}
                    </span>
                  </button>
                ))}
              </div>
            </div>
          )
        })}
      </div>
    </div>
  )
}
