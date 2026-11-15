import {
  useContext,
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
  type HTMLAttributes,
  type KeyboardEvent,
  type MouseEvent,
  type ReactNode,
} from 'react'
import { cn, useLocale } from 'dowel-ui'
import { BigCalendarDayContext } from './big-calendar-item'
import { addDays, firstDayOfWeek, keyStep, monthWeeks, parts, today, weekdayNames, weekRange, type IsoDate } from './calendar-math'
import { ScrollArea } from './scroll-area'

/*
 * BigCalendar.
 *
 * A month at a time, with what is on each day of it, filling the box it is
 * given.
 *
 * The question a month view is read for is not "what is on the 14th" - a
 * list answers that - but "is that week empty". So the month holds its box:
 * the weeks share the height, a day never grows past its share, and what a
 * day has no room for folds into a count. The count opens the day in place,
 * at the same height with its lines scrolling inside it, so a busy Friday does
 * not push the following week off the screen. One day open at a time: several
 * and the grid stops being a month at a glance, which is the only reason the
 * days fold.
 *
 * What a day holds is the product's: `renderDay` returns its lines, one node
 * each - BigCalendarItem draws one - and the grid decides how many show. The
 * sum is exact rather than estimated, because every line is twenty-four pixels and
 * they stand edge to edge: the grid measures one day's room and divides.
 *
 * Only the weeks the month touches are drawn, four to six, and the days of the
 * neighbouring months in them are real days on a quieter ground - a release
 * can be dragged onto the 31st of August from the September page. Today is
 * filled with the accent rather than coloured: on a grid of thirty-five cells
 * a coloured digit is not where the eye starts.
 *
 * A week is the same grid cut shorter: `week` instead of `month`, and the
 * grid draws whole weeks from the one that day is in - one by default, two
 * for a fortnight - with each day the whole height of the box. kilna kept a
 * row of days of its own for its week view until this was here; the days
 * are the month's, the same lines and the same fold, so they belong to the
 * same component. No day of a week is on a quieter ground: the neighbouring
 * month is a month view's idea.
 *
 * The month's name and the arrows that turn it are not here. They belong to
 * the screen's head, in one row with its filters and actions, which is where
 * kilna moved them; the grid asks for a new month only when the keyboard walks
 * off its edge.
 *
 * The keyboard is the Calendar's: the grid is one tab stop, the arrows move a
 * cursor through the days, PageUp and PageDown turn the month, and Enter
 * presses the day. Tab from a day goes into it - its add button, then its
 * lines - and then out of the month, because the lines of every other day are
 * out of the tab order: a month holding forty releases is not forty stops.
 *
 * Dragging is the product's. Every day carries `data-day`, so whatever library
 * moves things finds the day under the pointer with `dayOf`, and the handlers
 * go on the calendar itself rather than on forty-two cells. `dropTarget` lights
 * the day something would land on, and `dropHint` says what landing there
 * would mean.
 */

/** How tall a line of a day is, in pixels: BigCalendarItem's `h-6`. Lines
 * stand edge to edge, so this is the pitch as well - the floor of a pointer
 * target, with the chip drawn twenty pixels tall inside it. A product drawing
 * its own line draws it this tall. */
export const LINE_HEIGHT = 24

/* What a day is assumed to hold before it has been measured - the first frame,
 * and every test, since jsdom lays nothing out. Three lines are what a week
 * is at its least. */
const UNMEASURED_LINES = 3

/** How many lines fit in `height` pixels of a day. */
export function linesThatFit(height: number): number {
  if (!Number.isFinite(height) || height <= 0) return 0
  return Math.floor(height / LINE_HEIGHT)
}

/** What a day of `count` lines draws in `lines` of room: the first `shown`,
 * then `more` folded into a count. The count takes a line of its own, so a day
 * that cannot show everything gives up one more than the overflow - five in
 * four lines is three and "2 more", never four and a count cut off at the
 * foot of the day. */
export function foldDay(count: number, lines: number): { shown: number; more: number } {
  if (count <= lines) return { shown: count, more: 0 }
  const shown = Math.max(lines - 1, 0)
  return { shown, more: count - shown }
}

/** The lines of a day with a space between each two. A day is named by its
 * lines, and lines that are not buttons are blocks whose words a reader's
 * name runs together - "ClipSingle" - while the face shows them apart, which
 * axe reports as a name that does not match the words on screen. The day is a
 * flex column, which draws a space-only run as nothing. */
function spaced(lines: readonly ReactNode[]): ReactNode[] {
  return lines.flatMap((line, index) => (index === 0 ? [line] : [' ', line]))
}

/** The day an element is in, or `null` - for the target of a drop, or for
 * `document.elementFromPoint` under a pointer that is carrying something. */
export function dayOf(target: EventTarget | null): IsoDate | null {
  if (!(target instanceof Element)) return null
  return target.closest<HTMLElement>('[data-day]')?.dataset.day ?? null
}

type AddDay =
  | {
      /** Add something on this day: a plus in the corner of the day under
       * the pointer, and the first stop of Tab from the day the cursor is on. */
      onAddDay: (date: IsoDate) => void
      /** Names the plus. Required with it: an icon with no name announces
       * nothing. */
      addDayLabel: string
    }
  | { onAddDay?: undefined; addDayLabel?: undefined }

/* What the grid shows: a month, or whole weeks. One or the other, and each
 * with the word that turns it - a month view asked for a new week, or a week
 * view for a new month, would be a state nothing can draw. */
type Shown =
  | {
      /** Any day of the month to show. */
      month: IsoDate
      /** Asked for when the keyboard walks off the grid; any day of the new
       * month. Without it the cursor stops at the edge. */
      onMonthChange?: (month: IsoDate) => void
      week?: undefined
      weeks?: undefined
      onWeekChange?: undefined
    }
  | {
      /** Any day of the first week to show. With it the grid draws whole
       * weeks rather than a month, each day the whole height of the box. */
      week: IsoDate
      /** How many weeks from that one: one by default, two for a fortnight. */
      weeks?: number
      /** Asked for when the keyboard walks off the weeks, or PageUp and
       * PageDown turn them: any day of the new first week, a whole range on
       * from the one shown. Without it the cursor stops at the edge. */
      onWeekChange?: (week: IsoDate) => void
      month?: undefined
      onMonthChange?: undefined
    }

export type BigCalendarProps = AddDay &
  Shown &
  Omit<HTMLAttributes<HTMLDivElement>, 'children' | 'onSelect'> & {
    /** What a day holds, one node per line, each with a `key`. */
    renderDay: (date: IsoDate) => readonly ReactNode[]
    /** The words of the count a full day folds into, given how many are
     * folded: "3 more". */
    moreLabel: (count: number) => string
    /** The words that fold an open day again. */
    fewerLabel: string
    /** Pressing a day - a click on its ground, or Enter on the cursor. */
    onDaySelect?: (date: IsoDate) => void
    /** The day something being carried would land on. */
    dropTarget?: IsoDate | null
    /** What landing on `dropTarget` would mean, said at the foot of it. */
    dropHint?: ReactNode
    /** Names the months and the weekdays, and picks the first day of the week:
     * `en-GB` starts on Monday, `en-US` on Sunday, `en-u-fw-mon` on Monday
     * whatever the region. The application's language by default. */
    locale?: string
  }

/* The controls inside a day, which press themselves; the day's own press is
 * for its ground. */
const CONTROL = 'button, a, input, select, textarea, [role="button"]'

export function BigCalendar({
  month,
  onMonthChange,
  week,
  weeks: weekCount = 1,
  onWeekChange,
  renderDay,
  moreLabel,
  fewerLabel,
  onDaySelect,
  onAddDay,
  addDayLabel,
  dropTarget = null,
  dropHint,
  locale,
  className,
  onKeyDown,
  'aria-label': ariaLabel,
  ...props
}: BigCalendarProps) {
  const language = useLocale(locale)
  const start = firstDayOfWeek(language)
  // The day the product named: of the month, or of the first week. The type
  // gives exactly one of the two; it cannot see that through destructuring.
  const anchor = (week ?? month) as IsoDate
  const weeks = useMemo(
    () => (week === undefined ? monthWeeks(anchor, language) : weekRange(anchor, weekCount, language)),
    [anchor, week, weekCount, language],
  )
  const names = useMemo(() => weekdayNames(language, start), [language, start])
  const format = useMemo(
    () => ({
      heading: new Intl.DateTimeFormat(language, { month: 'long', year: 'numeric' }),
      range: new Intl.DateTimeFormat(language, { day: 'numeric', month: 'long', year: 'numeric' }),
      day: new Intl.DateTimeFormat(language, { dateStyle: 'full' }),
    }),
    [language],
  )
  const asDate = (date: IsoDate) => new Date(parts(date).year, parts(date).month - 1, parts(date).day)

  const days = weeks.flat()
  const first = days[0]!
  const last = days[days.length - 1]!
  const shown = parts(anchor)
  // A week view has no neighbouring month: every day in it is in range.
  const inRange = (date: IsoDate) =>
    week === undefined ? parts(date).year === shown.year && parts(date).month === shown.month : days.includes(date)
  const now = today()
  const id = useId()

  /* The keyboard's cursor. When the product turns the month with its own
   * arrows the old cursor is off the grid, and the keyboard starts again on
   * today if today is in range, or on the day the product named. */
  const [cursor, setCursor] = useState<IsoDate>(now)
  const active = days.includes(cursor) ? cursor : inRange(now) ? now : anchor

  const grid = useRef<HTMLDivElement>(null)
  const refocus = useRef(false)
  useEffect(() => {
    if (!refocus.current) return
    refocus.current = false
    grid.current?.querySelector<HTMLElement>(`[data-day="${active}"]`)?.focus()
  })

  /* How many lines a day has room for. Every day is the same height, so one
   * measurement answers for all of them, taken again whenever the box
   * changes. jsdom has no ResizeObserver, and lays nothing out to measure. */
  const [lines, setLines] = useState(UNMEASURED_LINES)
  useEffect(() => {
    const element = grid.current
    if (element === null || typeof ResizeObserver === 'undefined') return
    const observer = new ResizeObserver(() => {
      const room = element.querySelector<HTMLElement>('[data-day-lines]')
      if (room !== null) setLines(linesThatFit(room.clientHeight))
    })
    observer.observe(element)
    return () => observer.disconnect()
  }, [])

  const [open, setOpen] = useState<IsoDate | null>(null)

  const moveTo = (date: IsoDate) => {
    setCursor(date)
    refocus.current = true
  }

  const keyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    onKeyDown?.(event)
    if (event.defaultPrevented) return

    // Escape folds the open day from anywhere inside it.
    if (event.key === 'Escape' && open !== null && dayOf(event.target) === open) {
      event.preventDefault()
      setOpen(null)
      moveTo(open)
      return
    }

    // The rest belongs to the day itself, not a line inside it: a line may
    // want the arrows.
    const from = (event.target as HTMLElement).getAttribute('data-day')
    if (from === null) return
    if ((event.key === 'Enter' || event.key === ' ') && onDaySelect) {
      event.preventDefault()
      onDaySelect(from)
      return
    }

    // PageUp and PageDown turn what is shown: a month in a month, and the
    // range in a week view - a week, or a fortnight - rather than a month of
    // weeks the grid never drew.
    const page = event.key === 'PageDown' ? 1 : event.key === 'PageUp' ? -1 : 0
    const next = week !== undefined && page !== 0 ? addDays(from, page * 7 * weeks.length) : keyStep(event.key, from, start)
    if (next === undefined) return
    event.preventDefault()
    if (!days.includes(next)) {
      // Off the grid. What is shown is the product's, so it is asked to turn.
      if (week === undefined) {
        if (!onMonthChange) return
        onMonthChange(next)
      } else {
        if (!onWeekChange) return
        // A whole range on, back or forward, so the cursor lands inside it
        // whichever key took it off: no key moves it further than a range.
        onWeekChange(addDays(first, (next < first ? -7 : 7) * weeks.length))
      }
    }
    moveTo(next)
  }

  const pressDay = (date: IsoDate) => (event: MouseEvent<HTMLDivElement>) => {
    const control = (event.target as Element).closest(CONTROL)
    if (control !== null && event.currentTarget.contains(control)) return
    onDaySelect?.(date)
  }

  return (
    <div
      ref={grid}
      role="grid"
      aria-label={
        ariaLabel ??
        (week === undefined ? format.heading.format(asDate(anchor)) : format.range.formatRange(asDate(first), asDate(last)))
      }
      onKeyDown={keyDown}
      className={cn(
        'flex min-h-0 min-w-0 flex-col overflow-hidden rounded-lg border border-line bg-raise',
        className,
      )}
      {...props}
    >
      <div role="rowgroup">
        <div role="row" className="grid grid-cols-7 border-b border-line">
          {names.map((name) => (
            <div key={name} role="columnheader" className="caption truncate px-2 py-1.5">
              {name}
            </div>
          ))}
        </div>
      </div>

      <div role="rowgroup" className="grid min-h-0 flex-1 auto-rows-fr">
        {weeks.map((week, row) => (
          <div key={week[0]} role="row" className="grid min-h-0 grid-cols-7">
            {week.map((date, column) => {
              const items = renderDay(date)
              const fold = foldDay(items.length, lines)
              const isOpen = open === date && fold.more > 0
              const outside = !inRange(date)
              const target = dropTarget === date

              return (
                <div
                  key={date}
                  role="gridcell"
                  data-day={date}
                  data-outside={outside ? '' : undefined}
                  data-drop-target={target ? '' : undefined}
                  aria-current={date === now ? 'date' : undefined}
                  // Named by what it draws, in the order it draws it: the
                  // number, the lines, and the hint while something is carried
                  // over it. A name that opened with the whole date read
                  // "Monday, 14 September 2026 Clip" under a face reading
                  // "14 Clip" - not the words drawn, in their order, which is
                  // what someone telling voice control what they see says
                  // (WCAG 2.5.3). The whole date is the description. Named
                  // from its content, every day would also say "Add a release
                  // on this day", thirty-five times over.
                  aria-labelledby={`${id}${date}d ${id}${date}l${target && dropHint != null ? ` ${id}${date}h` : ''}`}
                  aria-describedby={`${id}${date}n`}
                  tabIndex={date === active ? 0 : -1}
                  onFocus={() => setCursor(date)}
                  onClick={onDaySelect && pressDay(date)}
                  className={cn(
                    'group/day relative flex min-h-0 min-w-0 flex-col gap-1 overflow-hidden p-1 transition-colors',
                    'focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent',
                    column < 6 && 'border-r border-line',
                    row < weeks.length - 1 && 'border-b border-line',
                    outside && 'bg-softer',
                    onDaySelect && 'cursor-pointer hover:bg-soft',
                    target && 'bg-accent-soft hover:bg-accent-soft',
                  )}
                >
                  {/* The day in full for whoever cannot see the grid: "14" on
                    * its own is not a date. */}
                  <span id={`${id}${date}n`} className="sr-only">
                    {format.day.format(asDate(date))}
                  </span>

                  <div className="flex h-5 shrink-0 items-center justify-between gap-1">
                    <span
                      id={`${id}${date}d`}
                      aria-hidden
                      className={cn(
                        'rounded-xs px-1 font-mono text-2xs tabular-nums',
                        date === now
                          ? 'bg-accent font-semibold text-on-accent'
                          : outside
                            ? 'text-faint'
                            : 'text-dim',
                      )}
                    >
                      {parts(date).day}
                    </span>

                    {/* Under the pointer, and on the day the keyboard is in: a
                      * plus on every cell is thirty-five plus signs to read
                      * past. Transparent rather than absent, so the row does
                      * not reflow as the pointer crosses the month. */}
                    {onAddDay && (
                      <button
                        type="button"
                        tabIndex={date === active ? 0 : -1}
                        aria-label={addDayLabel}
                        title={addDayLabel}
                        onClick={() => onAddDay(date)}
                        className={cn(
                          'target-min flex size-5 items-center justify-center rounded-sm text-dim opacity-0 transition-opacity',
                          'hover:bg-soft hover:text-text group-hover/day:opacity-100 group-focus-within/day:opacity-100',
                          'focus-visible:outline-2 focus-visible:outline-accent',
                        )}
                      >
                        <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden>
                          <path d="M8 3v10M3 8h10" fill="none" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" />
                        </svg>
                      </button>
                    )}
                  </div>
                  {/* A space between the number and the lines: text read off
                    * the face would otherwise run them together, "14Clip". */}
                  {' '}
                  <BigCalendarDayContext.Provider value={date === active}>
                    {isOpen ? (
                      <ScrollArea label={format.day.format(asDate(date))} orientation="vertical" className="flex-1">
                        <div id={`${id}${date}l`} className="flex flex-col">
                          {spaced(items)}
                          <Fold onClick={() => { setOpen(null); moveTo(date) }}>{fewerLabel}</Fold>
                        </div>
                      </ScrollArea>
                    ) : (
                      <div
                        // The room `lines` is measured from: the same in every day.
                        data-day-lines
                        id={`${id}${date}l`}
                        className="flex min-h-0 flex-1 flex-col overflow-hidden"
                      >
                        {spaced(items.slice(0, fold.shown))}
                        {fold.more > 0 && (
                          <Fold onClick={() => { setOpen(date); moveTo(date) }}>{moreLabel(fold.more)}</Fold>
                        )}
                      </div>
                    )}
                  </BigCalendarDayContext.Provider>

                  {/* Over the foot of the day rather than under its lines: the
                    * day does not grow, and the hint is only up while something
                    * is carried over it. */}
                  {target && dropHint != null && (
                    <p
                      id={`${id}${date}h`}
                      className="pointer-events-none absolute inset-x-1 bottom-1 rounded-sm bg-raise px-1 py-0.5 text-2xs leading-tight text-dim shadow-lift"
                    >
                      {dropHint}
                    </p>
                  )}
                </div>
              )
            })}
          </div>
        ))}
      </div>
    </div>
  )
}

/** The line that folds a day or opens it: a line's height, so the sum holds,
 * and a target by its own size. */
function Fold({ onClick, children }: { onClick: () => void; children: string }) {
  const inCursorDay = useContext(BigCalendarDayContext)
  return (
    <button
      type="button"
      tabIndex={inCursorDay === false ? -1 : 0}
      onClick={onClick}
      className={cn(
        'h-6 min-w-6 shrink-0 self-start rounded-sm px-1 text-xs text-dim hover:text-text',
        'focus-visible:outline-2 focus-visible:-outline-offset-1 focus-visible:outline-accent',
      )}
    >
      {children}
    </button>
  )
}
