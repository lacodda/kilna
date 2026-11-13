import { createContext, useContext, type CSSProperties, type HTMLAttributes, type ReactNode } from 'react'
import { useRender } from '@base-ui/react/use-render'
import { cn } from 'dowel-ui'

/*
 * BigCalendarItem.
 *
 * One thing on a day of a month view, in one line: a release, a lesson, a
 * payment that falls due.
 *
 * One line is the rule the month rests on. BigCalendar folds what a day has
 * no room for into a count, and the count is arithmetic - lines of
 * twenty-four pixels, edge to edge - so an item that wraps to two lines on a narrow day
 * breaks the sum and pushes the last line out of the day. The words truncate
 * instead; the whole of them is in `title`, or wherever the product puts a
 * hover card.
 *
 * The colour is the item's own, not the product's: the work it belongs to, the
 * category it is in. Any CSS colour is taken - `var(--series-3)` for a
 * category, the work's own colour where a product keeps one - and drawn as a
 * soft fill with ink leaning towards the theme's text, so the same colour
 * reads on the dark theme and the light one. Without a colour the item is
 * neutral.
 *
 * Two states change how it is drawn, because both are said by kilna's month
 * every day: `tentative` is a plan rather than a booking - a dashed outline,
 * so a suggestion never reads as a fact - and `done` has happened, drawn
 * quieter and still openable, and still readable: a weaker fill and dim ink,
 * not a faded line.
 *
 * It stands outside the month as well: the queue of things waiting for a date
 * beside it, and the copy that travels under the pointer while one is dragged,
 * are the same line.
 *
 * The line is twenty-four pixels and the chip drawn in it is twenty. The box
 * is the pitch of the month - lines stand edge to edge, and the chips come out
 * four apart - so the target is the floor a pointer is asked to hit (WCAG
 * 2.5.8) by its own size, not by an invisible extension: inside a day that is
 * itself a target, a measurement of the box is the only one anything takes.
 * The element is the line; its colour, outline and ring are the chip's.
 */

/**
 * Whether a line stands in the day the month's keyboard is on: `true` there,
 * `false` in every other day, `null` outside a month.
 *
 * The month is one tab stop and the arrows walk its days; Tab from a day goes
 * into that day and then out of the month. So only the lines of the day under
 * the cursor are in the tab order - a month holding forty releases is not
 * forty tab stops. BigCalendarItem reads this itself; a product drawing its
 * own line reads it for the line's `tabIndex`.
 */
export const BigCalendarDayContext = createContext<boolean | null>(null)

export interface BigCalendarItemProps extends Omit<HTMLAttributes<HTMLElement>, 'color'> {
  /** The item's own colour - any CSS colour, `var(--series-3)` for a
   * category. Neutral without one. */
  color?: string
  /** A plan rather than a booking: a dashed outline. */
  tentative?: boolean
  /** It has happened: drawn quieter, still pressable. */
  done?: boolean
  /** Before the words - a cover, a dot. */
  start?: ReactNode
  /** After the words - the glyphs that mark it: a kind, a lock, a status.
   * They do not shrink; the words truncate. */
  end?: ReactNode
  /** The element to draw: the product's link, a hover card's trigger.
   * Without it the item is a `<button>` when it has `onClick`, and a `<div>`
   * when it has nothing to do. */
  render?: useRender.RenderProp
}

const slot = 'flex shrink-0 items-center gap-0.5 [&_svg:not([class*=size-])]:size-3'

/* Quieter is a weaker fill and an ink halfway from the text to dim - never
 * opacity. kilna drew a released chip at 60% and its words measured 2.7:1 in
 * the light theme, 1.7:1 on a day of the next month, which was dimmed again;
 * the series mixed into `--dim` alone still came to 3.55. Halfway clears AA
 * in every series of every product (`tests/palettes.test.ts` holds it). */
const QUIET_INK = 'color-mix(in oklab, var(--dim), var(--text))'

function tint(color: string | undefined, tentative: boolean, done: boolean): CSSProperties | undefined {
  if (tentative) return color === undefined ? undefined : { borderColor: color }
  if (color === undefined) return done ? { color: QUIET_INK } : undefined
  return {
    background: `color-mix(in oklab, ${color} ${done ? 15 : 30}%, transparent)`,
    color: `color-mix(in oklab, ${color} 30%, ${done ? QUIET_INK : 'var(--text)'})`,
  }
}

export function BigCalendarItem({
  color,
  tentative = false,
  done = false,
  start,
  end,
  render,
  className,
  style,
  children,
  onClick,
  ...props
}: BigCalendarItemProps) {
  const pressable = onClick !== undefined
  const inCursorDay = useContext(BigCalendarDayContext)
  return useRender({
    render,
    defaultTagName: pressable ? 'button' : 'div',
    props: {
      ...(render === undefined && pressable ? { type: 'button' } : {}),
      ...(inCursorDay === false ? { tabIndex: -1 } : {}),
      onClick,
      'data-tentative': tentative ? '' : undefined,
      'data-done': done ? '' : undefined,
      className: cn(
        // Twenty-four: the pitch of a month's lines, and the floor of a
        // pointer target. The two pixels above and below the chip are the
        // four between two chips.
        'group/line flex h-6 w-full min-w-0 shrink-0 items-stretch rounded-sm py-0.5 text-left text-xs',
        // Inside the box, so the ring hugs the chip rather than the gap.
        'focus-visible:outline-2 focus-visible:-outline-offset-1 focus-visible:outline-accent',
        pressable && 'cursor-pointer',
        className,
      ),
      style,
      children: (
        <span
          data-chip
          className={cn(
            'flex min-w-0 flex-1 items-center gap-1 rounded-sm px-1',
            tentative
              ? 'border border-dashed border-line-2 font-medium text-dim'
              : cn(
                  done ? 'font-medium' : 'font-semibold',
                  color === undefined && (done ? 'bg-softer' : 'bg-soft text-text'),
                ),
            pressable && 'group-hover/line:inset-ring group-hover/line:inset-ring-line-2',
          )}
          style={tint(color, tentative, done)}
        >
          {start === undefined ? null : <span className={slot}>{start}</span>}
          <span className="min-w-0 flex-1 truncate">{children}</span>
          {end === undefined ? null : <span className={slot}>{end}</span>}
        </span>
      ),
      ...props,
    },
  })
}
