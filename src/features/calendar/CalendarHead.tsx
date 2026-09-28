import { useTranslation } from 'react-i18next'
import { ChevronLeft, ChevronRight, ChevronsLeft, ChevronsRight, FilePenLine } from 'lucide-react'
import type { ScheduledRelease } from '@/lib/api/types'
import type { KindFilter } from '@/lib/calendarFilter'
import { otherLayout, type CalendarLayout } from '@/lib/calendarLayout'
import { formatMonth } from '@/lib/format'
import { monthOf, sameMonth, shiftMonth, today, type Month } from '@/lib/month'
import { Button } from '@/components/ui/button'
import { KindFilterBar } from '@/features/calendar/KindFilterBar'
import { QueueDrop } from '@/features/calendar/QueueDrop'
import type { DropTarget } from '@/features/calendar/useCalendarDrag'

interface Props {
  month: Month
  onMonthChange: (month: Month) => void
  /** Everything with a date, unfiltered: the chips count the whole calendar. */
  slots: readonly ScheduledRelease[]
  kind: KindFilter
  onKindChange: (kind: KindFilter) => void
  /** How many planned releases of the month the fields batch would write. */
  planned: number
  onFillFields: () => void
  fillingFields: boolean
  width: CalendarLayout
  onWidthChange: (width: CalendarLayout) => void
  /** Where a carried release goes back to the queue while the queue is hidden:
   * the full-width month has nowhere else to put it. Absent otherwise. */
  drop?: { over: boolean; target: DropTarget }
}

/**
 * The screen's head, in one row: the month and its arrows, the way back to
 * this month, the kinds the month shows, and what acts on the month as a
 * whole.
 *
 * Three rows stood here until v0.79 - the title and arrows over the grid, the
 * chips and the actions above them, the plan's banner above those - and the
 * month's name was the smallest text on the screen. The mockup's head is one
 * row with the title at its size. The row is a container: below a width that
 * holds everything, the fields action folds to its glyph (the words stay its
 * name and its tooltip), and only a window narrower still wraps the row -
 * never squeezing the title into two words.
 */
export function CalendarHead({
  month,
  onMonthChange,
  slots,
  kind,
  onKindChange,
  planned,
  onFillFields,
  fillingFields,
  width,
  onWidthChange,
  drop,
}: Props) {
  const { t } = useTranslation()
  const current = monthOf(today())
  const widen = width === 'queue' ? 'calendar.widen' : 'calendar.showQueue'

  return (
    <div className="@container flex w-full flex-wrap items-center gap-2">
      <Button
        variant="icon"
        size="icon-sm"
        aria-label={t('calendar.previousMonth')}
        title={t('calendar.previousMonth')}
        onClick={() => onMonthChange(shiftMonth(month, -1))}
      >
        <ChevronLeft aria-hidden />
      </Button>
      {/* A fixed width, so the arrows do not move under the pointer as the
          months' names change length. */}
      <h1 className="min-w-36 text-center text-lg font-semibold capitalize">
        {formatMonth(month.year, month.month)}
      </h1>
      <Button
        variant="icon"
        size="icon-sm"
        aria-label={t('calendar.nextMonth')}
        title={t('calendar.nextMonth')}
        onClick={() => onMonthChange(shiftMonth(month, 1))}
      >
        <ChevronRight aria-hidden />
      </Button>

      {/* Always there, and out of use on this month: a button that came and
          went moved everything after it along the row as the months turned. */}
      <Button size="sm" disabled={sameMonth(month, current)} onClick={() => onMonthChange(current)}>
        {t('calendar.thisMonth')}
      </Button>

      <KindFilterBar slots={slots} value={kind} onChange={onKindChange} />

      <div className="ml-auto flex items-center gap-2">
        {drop !== undefined ? (
          <QueueDrop active over={drop.over} target={drop.target} className="w-auto" />
        ) : (
          <>
            {/* Everything still planned in the month, written in one pass.
                It reads what the month holds rather than what the chips are
                showing: a narrowed view is a way of looking, not an
                instruction about which releases to write. */}
            {planned > 0 && (
              <Button
                size="sm"
                aria-label={t('calendar.fields.action')}
                title={t('calendar.fields.action')}
                disabled={fillingFields}
                onClick={onFillFields}
              >
                <FilePenLine aria-hidden />
                <span className="hidden @min-[68rem]:inline">{t('calendar.fields.action')}</span>
              </Button>
            )}

            {/* At the far end: it is about how much of the screen the month
                gets, and is not an action on a release. */}
            <Button
              variant="icon"
              size="icon-sm"
              aria-label={t(widen)}
              title={t(widen)}
              onClick={() => onWidthChange(otherLayout(width))}
            >
              {width === 'queue' ? <ChevronsRight aria-hidden /> : <ChevronsLeft aria-hidden />}
            </Button>
          </>
        )}
      </div>
    </div>
  )
}
