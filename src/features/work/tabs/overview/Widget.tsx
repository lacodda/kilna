import { createContext, useContext, useId, type MouseEvent, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate } from 'react-router'
import { ArrowRight } from 'lucide-react'
import type { WidgetSize } from '@/lib/api/types'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Panel } from '@/components/ui/panel'
import { Skeleton } from '@/components/ui/skeleton'
import { BOUND, Scroll } from '@/components/frame'

/*
 * One widget of the overview: the mockup's `.w`, and the one object every
 * layout draws.
 *
 * The anatomy is the widget catalogue's: a caption in small capitals, the
 * body - the fact itself - an "Open" that shows on hover and goes to the tab
 * that owns the fact, and a foot for the actions done without looking (copy).
 * Nothing else, or the board stops reading as one board.
 *
 * A widget carries every presentation of its fact with it, so a layout never
 * needs a widget of its own (the catalogue's rule): the grid, the lead column
 * and the mosaic draw cards; the bands draw each as a row across the board,
 * and the sheet as a line of one document. The layout says which through
 * `WidgetLook`, and says how large the widget stands, so a text given two
 * cells by two reads longer than one given a single cell.
 *
 * A card or a band is never taller than the board shows (`BOUND`): what it
 * says past that scrolls inside it, under a caption that stands. A list of
 * ten repeats beside the stage and the links scrolled the whole board until
 * v0.90.1, and the rail went up with it; now the board scrolls only when its
 * widgets together do not fit.
 */

/** How a layout draws its widgets: as cards, as bands across, as lines of a sheet. */
export type Presentation = 'card' | 'band' | 'line'

export interface Look {
  presentation: Presentation
  size: WidgetSize
  /** Where the layout puts the widget: the cells it spans, the height it
   *  starts at. On the widget's own box, so a layout needs no wrapper. */
  place?: string
}

export const WidgetLook = createContext<Look>({ presentation: 'card', size: 's' })

export function useLook(): Look {
  return useContext(WidgetLook)
}

/**
 * A widget's condition, beyond its data: `empty` is an invitation - drawn
 * dashed, with the button that fills it - and `attention` is something that
 * asks for the person rather than telling them, in a warm border, never red.
 */
export type WidgetTone = 'plain' | 'empty' | 'attention'

interface Props {
  caption: ReactNode
  /** The tab that owns this fact. The widget goes there when clicked. */
  to?: string
  /** What the way there says; "Open" unless the tab is worth naming. */
  go?: string
  /** Beside the caption: a state, a control that belongs to the whole widget. */
  aside?: ReactNode
  /** Done without leaving the board - copy, edit, the button of an invitation. */
  actions?: ReactNode
  tone?: WidgetTone
  className?: string
  children?: ReactNode
}

/**
 * What a click on the widget must leave alone: a control inside it; a click
 * in a popup it opened, which React hands up the tree it was declared in
 * though it is drawn elsewhere; and a click that ended a selection - text on a
 * widget is there to be copied too, and selecting a line must not carry the
 * person off to another tab.
 */
function leaveAlone(event: MouseEvent<HTMLElement>): boolean {
  const target = event.target as Element
  if (!event.currentTarget.contains(target)) return true
  if (target.closest('a, button, input, textarea, label, [role="button"], [role="slider"]')) {
    return true
  }
  // The bar of the body's own scroll: dragged, not pressed.
  if (target.closest('[data-id$="-scrollbar"]')) return true
  return (window.getSelection()?.toString() ?? '') !== ''
}

export function Widget({
  caption,
  to,
  go,
  aside,
  actions,
  tone = 'plain',
  className,
  children,
}: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { presentation, place } = useLook()
  const id = useId()
  const way = go ?? t('overview.open')
  // What the body's scroll region is called, for the tab stop it becomes when
  // it overflows: the widget in full - the widget itself is already a group
  // named by its caption, and two groups of one name read as one twice.
  const label = t('overview.inFull', {
    caption: typeof caption === 'string' ? caption : t('card.tab.overview'),
  })

  // A click on the widget goes to the tab that owns it, not to a modal (the
  // owner's rule): the text lives on Versions, and a copy of the editor here
  // would give it two homes. The link in the caption is the same way there
  // for the keyboard and a screen reader.
  const onClick =
    to === undefined
      ? undefined
      : (event: MouseEvent<HTMLElement>) => {
          if (!leaveAlone(event)) void navigate(to)
        }

  const heading = (
    <h2
      id={id}
      className={cn(
        'caption min-w-0 truncate',
        presentation === 'card' && 'flex-1',
        presentation === 'line' && 'pt-1',
        tone === 'attention' && 'text-warn',
      )}
    >
      {caption}
    </h2>
  )

  if (presentation === 'card') {
    return (
      <Panel
        role="group"
        aria-labelledby={id}
        onClick={onClick}
        // A grid rather than a column: the caption, the body in what is left
        // of the widget's height, the actions. The body's row is what gives
        // its scroll region a height to scroll against once the widget meets
        // its bound; short of that it is as tall as what it says. The sides
        // are padded row by row, so the body's bar runs down the border.
        className={cn(
          'group/widget grid min-w-0 grid-rows-[auto_minmax(0,1fr)] gap-2 py-2.5 transition-colors hover:border-line-2',
          BOUND,
          to !== undefined && 'cursor-pointer',
          tone === 'empty' && 'border-dashed bg-softer',
          tone === 'attention' && 'border-warn/45 hover:border-warn/70',
          place,
          className,
        )}
      >
        <header className="flex min-w-0 items-center gap-2 px-3">
          {heading}
          {aside}
          {to !== undefined && (
            // Shown on hover, as the catalogue draws it: the way out is not
            // what the widget says, so it does not stand in every caption.
            // Still in the tab order, and shown when reached by it.
            <Link
              to={to}
              className="inline-flex shrink-0 items-center gap-1 text-xs text-faint no-underline opacity-0 transition-opacity group-hover/widget:opacity-100 hover:text-text focus-visible:opacity-100"
            >
              {way}
              <ArrowRight aria-hidden className="size-3" />
            </Link>
          )}
        </header>
        <Scroll label={label} contentClassName="flex min-w-0 flex-col gap-2 px-3">
          {children}
        </Scroll>
        {actions !== undefined && (
          <div className="flex flex-wrap items-center gap-1.5 px-3">{actions}</div>
        )}
      </Panel>
    )
  }

  const goButton =
    to === undefined ? null : (
      <Button size="xs" render={<Link to={to} />}>
        {way}
        <ArrowRight aria-hidden />
      </Button>
    )

  if (presentation === 'band') {
    // The mockup's `.band`: a caption column, the fact, and what acts on it
    // at the end. Narrow, the three stack.
    return (
      <Panel
        role="group"
        aria-labelledby={id}
        className={cn(
          'grid min-w-0 grid-cols-[118px_minmax(0,1fr)_auto] items-center gap-3.5 px-3.5 py-2.5 transition-colors hover:border-line-2',
          '@max-[47.5rem]:grid-cols-1 @max-[47.5rem]:gap-2',
          tone === 'empty' && 'border-dashed bg-softer',
          tone === 'attention' && 'border-warn/45',
          place,
          className,
        )}
      >
        {heading}
        {/* The fact in its column, as tall as the board less the band's own
            padding at most: a long one scrolls there, and the caption and the
            actions stand beside it. */}
        <Scroll
          label={label}
          fit
          className="max-h-[calc(100cqh-var(--bound-inset,0px)-1.25rem)]"
          contentClassName="min-w-0"
        >
          {children}
        </Scroll>
        <div className="flex shrink-0 flex-wrap items-center gap-1.5">
          {aside}
          {actions}
          {goButton}
        </div>
      </Panel>
    )
  }

  // A line of the sheet: one document, so no box of its own - a caption
  // column and the fact, divided from the next line by a hairline. The sheet
  // draws the box around all of them.
  return (
    <div
      role="group"
      aria-labelledby={id}
      className={cn(
        'grid min-w-0 grid-cols-[132px_minmax(0,1fr)] items-start gap-4 border-b border-line px-4 py-2.5 transition-colors last:border-b-0 hover:bg-softer',
        '@max-[44rem]:grid-cols-1 @max-[44rem]:gap-1',
        place,
        className,
      )}
    >
      {heading}
      <div className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1.5">
        <div className="min-w-0 flex-1">{children}</div>
        {(aside !== undefined || actions !== undefined || goButton !== null) && (
          <div className="ml-auto flex shrink-0 flex-wrap items-center gap-1.5">
            {aside}
            {actions}
            {goButton}
          </div>
        )}
      </div>
    </div>
  )
}

/** What an empty widget says will be here. The button that makes it happen
 *  goes in the widget's `actions`, where every layout puts its buttons. */
export function Invite({ children }: { children: ReactNode }) {
  return <p className="text-sm text-faint">{children}</p>
}

/** The button of an invitation: the way to the tab where the thing is made. */
export function InviteLink({ to, children }: { to: string; children: ReactNode }) {
  return (
    <Button size="xs" variant="soft" render={<Link to={to} />}>
      {children}
    </Button>
  )
}

/**
 * A widget still loading: the shape of what is coming, about the height of
 * it, so the board does not jump when it lands.
 */
export function WidgetSkeleton({ lines = 2 }: { lines?: number }) {
  const { presentation } = useLook()
  if (presentation !== 'card') return <Skeleton className="h-4 w-3/5" />
  return (
    <div className="flex flex-col gap-2" aria-hidden>
      <Skeleton className="h-6 w-1/2" />
      {Array.from({ length: Math.max(lines - 1, 0) }, (_, index) => (
        <Skeleton key={index} className="h-3 w-3/4" />
      ))}
    </div>
  )
}
