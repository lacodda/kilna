import { useTranslation } from 'react-i18next'
import {
  ChevronLeft,
  ChevronRight,
  ChevronsLeft,
  ChevronsRight,
  FilePenLine,
  MoreHorizontal,
} from 'lucide-react'
import type { ScheduledRelease } from '@/lib/api/types'
import type { KindFilter } from '@/lib/calendarFilter'
import { otherLayout, type CalendarLayout } from '@/lib/calendarLayout'
import { inPeriod, periodOf, stepAnchor, VIEWS, type CalendarView } from '@/lib/calendarPeriod'
import { formatDay, formatMonth } from '@/lib/format'
import { monthOf, today } from '@/lib/month'
import { Button } from '@/components/ui/button'
import { Menu, MenuItem, MenuPopup, MenuSeparator, MenuTrigger } from '@/components/ui/menu'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { KindFilterBar } from '@/features/calendar/KindFilterBar'
import { QueueDrop } from '@/features/calendar/QueueDrop'
import type { DropTarget } from '@/features/calendar/useCalendarDrag'

interface Props {
  view: CalendarView
  onViewChange: (view: CalendarView) => void
  /** The day the screen is on: the views show its month or its week. */
  anchor: string
  onAnchorChange: (anchor: string) => void
  /** Everything with a date, unfiltered: the chips count the whole calendar. */
  slots: readonly ScheduledRelease[]
  kind: KindFilter
  onKindChange: (kind: KindFilter) => void
  /** How many planned releases of the period the fields batch would write. */
  planned: number
  onFillFields: () => void
  fillingFields: boolean
  /** The days the calendar is closed, and what closing them asks. */
  onTimeOff: () => void
  /** Shift the plan from a day on. */
  onShift: () => void
  /** Keep, or free, every planned date of the period on screen. */
  onPinPeriod: (pinned: boolean) => void
  pinning: boolean
  /** The month on screen as a Markdown table, every dated release as an
      iCalendar file, and a plan read from a CSV file (v0.104). */
  onExportMonth: () => void
  onExportIcs: () => void
  onImportPlan: () => void
  width: CalendarLayout
  onWidthChange: (width: CalendarLayout) => void
  /** Where a carried release goes back to the queue while the queue is hidden:
   * the full-width month has nowhere else to put it. Absent otherwise. */
  drop?: { over: boolean; target: DropTarget }
}

/**
 * The screen's head, in one row: the period and its arrows, the way back to
 * today, how the plan is looked at, the kinds it shows, and what acts on the
 * period as a whole.
 *
 * Three rows stood here until v0.79 - the title and arrows over the grid, the
 * chips and the actions above them, the plan's banner above those - and the
 * month's name was the smallest text on the screen. The mockup's head is one
 * row with the title at its size. The row is a container: below a width that
 * holds everything, the fields action folds to its glyph (the words stay its
 * name and its tooltip), and only a window narrower still wraps the row -
 * never squeezing the title into two words.
 *
 * What acts on the calendar rather than on a release - the closed days, a
 * shift, keeping a period's dates (v0.96) - is one menu at the end: done now
 * and then, and a row of buttons for each would push the kinds off the head.
 */
export function CalendarHead({
  view,
  onViewChange,
  anchor,
  onAnchorChange,
  slots,
  kind,
  onKindChange,
  planned,
  onFillFields,
  fillingFields,
  onTimeOff,
  onShift,
  onPinPeriod,
  pinning,
  onExportMonth,
  onExportIcs,
  onImportPlan,
  width,
  onWidthChange,
  drop,
}: Props) {
  const { t } = useTranslation()
  const now = today()
  const period = periodOf(view, anchor)
  const widen = width === 'queue' ? 'calendar.widen' : 'calendar.showQueue'
  const week = view === 'week'
  const { year, month } = monthOf(anchor)
  const title = week
    ? `${formatDay(period.from)} – ${formatDay(period.to)}`
    : formatMonth(year, month)
  const previous = week ? 'calendar.previousWeek' : 'calendar.previousMonth'
  const next = week ? 'calendar.nextWeek' : 'calendar.nextMonth'

  return (
    <div className="@container flex w-full flex-wrap items-center gap-2">
      <Button
        variant="icon"
        size="icon-sm"
        aria-label={t(previous)}
        title={t(previous)}
        onClick={() => onAnchorChange(stepAnchor(view, anchor, -1))}
      >
        <ChevronLeft aria-hidden />
      </Button>
      {/* A fixed width, so the arrows do not move under the pointer as the
          months' names change length. The first letter up, not every word's:
          "Октябрь 2026 г.", never "Октябрь 2026 Г." or "5 Окт. – 11 Окт.". */}
      <h1 className="min-w-36 text-center text-lg font-semibold first-letter:uppercase">{title}</h1>
      <Button
        variant="icon"
        size="icon-sm"
        aria-label={t(next)}
        title={t(next)}
        onClick={() => onAnchorChange(stepAnchor(view, anchor, 1))}
      >
        <ChevronRight aria-hidden />
      </Button>

      {/* Always there, and out of use on the period holding today: a button
          that came and went moved everything after it along the row. */}
      <Button size="sm" disabled={inPeriod(now, period)} onClick={() => onAnchorChange(now)}>
        {t('calendar.today')}
      </Button>

      <SegmentedControl
        aria-label={t('calendar.view.label')}
        value={view}
        onValueChange={(nextView) => onViewChange(nextView as CalendarView)}
      >
        {VIEWS.map((one) => (
          <Segment key={one} value={one}>
            {t(`calendar.view.${one}`)}
          </Segment>
        ))}
      </SegmentedControl>

      <KindFilterBar slots={slots} value={kind} onChange={onKindChange} />

      <div className="ml-auto flex items-center gap-2">
        {drop !== undefined ? (
          <QueueDrop active over={drop.over} target={drop.target} className="w-auto" />
        ) : (
          <>
            {/* Everything still planned in the period, written in one pass.
                It reads what the period holds rather than what the chips are
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

            <Menu>
              <MenuTrigger
                render={
                  <Button
                    variant="icon"
                    size="icon-sm"
                    aria-label={t('calendar.more')}
                    title={t('calendar.more')}
                  />
                }
              >
                <MoreHorizontal aria-hidden />
              </MenuTrigger>
              <MenuPopup align="end">
                <MenuItem onClick={onTimeOff}>{t('timeOff.menu')}</MenuItem>
                <MenuItem onClick={onShift}>{t('shift.menu')}</MenuItem>
                <MenuSeparator />
                <MenuItem disabled={pinning} onClick={() => onPinPeriod(true)}>
                  {t(week ? 'calendar.pinWeek' : 'calendar.pinMonth')}
                </MenuItem>
                <MenuItem disabled={pinning} onClick={() => onPinPeriod(false)}>
                  {t(week ? 'calendar.unpinWeek' : 'calendar.unpinMonth')}
                </MenuItem>
                <MenuSeparator />
                <MenuItem onClick={onExportMonth}>{t('calendar.files.exportMonth')}</MenuItem>
                <MenuItem onClick={onExportIcs}>{t('calendar.files.exportIcs')}</MenuItem>
                <MenuItem onClick={onImportPlan}>{t('calendar.files.importPlan')}</MenuItem>
              </MenuPopup>
            </Menu>

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
