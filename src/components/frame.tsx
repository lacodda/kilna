import type { ReactNode, Ref } from 'react'
import { ScrollArea } from '@/components/ui/scroll-area'
import { cn } from '@/lib/utils'

/*
 * The frame every screen and every tab of a card is laid out in (ADR 0038).
 *
 * The window has a bottom edge and nothing runs past it. A screen is a column
 * that takes the height it is handed: a head that stands, a body that takes
 * the rest, sometimes a foot that stands too - and only a part of the body
 * scrolls, never the column. When the column scrolled, the head went up with
 * the content, a table's sticky header had nothing to stick to, and a field
 * at the foot of a tab drifted off the bottom of the window with everything
 * above it. The audit of 24.09 found eleven screens and tabs drawn that way
 * after v0.75 had said none were.
 *
 * Four pieces, because the screens are built of four shapes:
 *
 * - `Frame` - the column itself. It never scrolls.
 * - `Scroll` - the part of a frame that does, with the line's overlay bar:
 *   drawn over the content, taking no width, so a list that grows past the
 *   fold does not push everything beside it sideways.
 * - `Pane` - a panel with a bar at the top, a body that scrolls, and a bar at
 *   the foot: an open note, a version being read, a list with its filters.
 * - `ListDetail` - a list beside what is open in it. Six screens drew this by
 *   hand with five widths and three gaps; there are two widths now, a list's
 *   and a rail's, and one gap.
 *
 * The spacing is the mockup's: ten pixels between the blocks of a screen,
 * whichever of these holds them.
 *
 * A region of widgets - the dashboard's side column, a work's overview - is a
 * `Scroll` that is `bound`: each widget in it is as tall as the region at
 * most and scrolls its own body past that (`BOUND`), so the region itself
 * scrolls only when its widgets together do not fit. A widget taller than
 * the window moved everything beside it whenever it was read (v0.90.1).
 */

interface FrameProps {
  /** What stands above the body: a title, the filters, the screen's actions. */
  head?: ReactNode
  /** What stands under it: a composer, a status line. */
  foot?: ReactNode
  children: ReactNode
  className?: string
}

/** A screen's or a tab's column. Hands its height to the body and scrolls nowhere. */
export function Frame({ head, foot, children, className }: FrameProps) {
  return (
    <div data-frame className={cn('flex min-h-0 min-w-0 flex-1 flex-col gap-2.5', className)}>
      {head === undefined ? null : (
        <div className="flex shrink-0 flex-wrap items-center gap-2">{head}</div>
      )}
      <div className="flex min-h-0 min-w-0 flex-1 flex-col">{children}</div>
      {foot === undefined ? null : (
        <div className="flex shrink-0 flex-wrap items-center gap-2">{foot}</div>
      )}
    </div>
  )
}

interface ScrollProps {
  /** What the region holds - it becomes a tab stop when it overflows, and a
   * tab stop has to be called something. */
  label: string
  children: ReactNode
  /** On the region: its size, and anything that places it. */
  className?: string
  /** On the scrolling box: the padding the content sits in. */
  contentClassName?: string
  /** The scrolling element, for a caller that reads or sets the position. */
  viewportRef?: Ref<HTMLDivElement>
  /**
   * The region's height is what a block inside may take (`BOUND`): it becomes
   * a size container, so `cqh` inside reads its height. Only for a region
   * whose height its frame gives it - a size container is sized as if empty.
   */
  bound?: boolean
  /**
   * The region is as tall as what it holds, up to the `max-h-*` its class
   * gives it, rather than the rest of a column: for a region in a box whose
   * height is its content's - a band of the overview. The scrolling box
   * takes the same limit, since a height of 100% is nothing in such a box.
   */
  fit?: boolean
}

/**
 * The most a widget in a `bound` region may stand: the region's height, less
 * the padding the region's content sits in (`--bound-inset`, nothing unless
 * the region sets it). A widget given it scrolls its own body past it.
 */
export const BOUND = 'max-h-[calc(100cqh-var(--bound-inset,0px))]'

/** The part of a frame that scrolls. Takes the rest of the column it is in. */
export function Scroll({
  label,
  children,
  className,
  contentClassName,
  viewportRef,
  bound = false,
  fit = false,
}: ScrollProps) {
  return (
    <ScrollArea
      label={label}
      viewportRef={viewportRef}
      className={cn(!fit && 'flex-1', bound && '[container-type:size]', className)}
      viewportClassName={cn(
        // The content box is `min-width: fit-content`, so that a wide table
        // can overflow sideways. A frame scrolls down, not across: its rows
        // truncate to the column instead of pushing it wider - the queue's
        // rows ran a hundred pixels past a 264px column before this.
        '*:min-w-0!',
        fit && 'max-h-[inherit]',
      )}
    >
      {/* The content's own box, inside the one Base UI measures: the
          viewport's single child is Base UI's, so a gap or a column set on
          the viewport would reach nothing. */}
      <div className={contentClassName}>{children}</div>
    </ScrollArea>
  )
}

interface PaneProps {
  /** What the body holds, for the scroll region's name. */
  label: string
  /** The bar across the top: a title and what acts on the open thing. */
  head?: ReactNode
  /** The bar across the foot: tags, a status, a button that adds a row. */
  foot?: ReactNode
  children: ReactNode
  className?: string
  /** The body's padding. A list's rows pad themselves, prose needs room. */
  bodyClassName?: string
  /** For the body's scroll position. */
  viewportRef?: Ref<HTMLDivElement>
}

/** A panel whose bars stand and whose body scrolls between them. */
export function Pane({
  label,
  head,
  foot,
  children,
  className,
  bodyClassName,
  viewportRef,
}: PaneProps) {
  return (
    <section
      data-pane
      className={cn(
        'flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden rounded-lg border border-line bg-raise',
        className,
      )}
    >
      {head === undefined ? null : (
        <div className="flex shrink-0 flex-wrap items-center gap-2 border-b border-line px-3 py-2">
          {head}
        </div>
      )}
      <Scroll label={label} contentClassName={bodyClassName} viewportRef={viewportRef}>
        {children}
      </Scroll>
      {foot === undefined ? null : (
        <div className="flex shrink-0 flex-wrap items-center gap-2 border-t border-line px-3 py-2">
          {foot}
        </div>
      )}
    </section>
  )
}

interface ListDetailProps {
  /** The list, usually a `Pane` of `RowButton`s. */
  list: ReactNode
  /** What is open in it, or what says nothing is. */
  detail: ReactNode
  /**
   * `list` for rows that carry a title and a line under it - versions,
   * scores, notes, comments, a queue. `rail` for a column of destinations,
   * the width of the app's own rail - the sections of the settings.
   */
  width?: 'list' | 'rail'
  /** Which side the list stands on. The calendar's queue is read against the
   * month, which comes first. */
  side?: 'start' | 'end'
  className?: string
}

/** A list beside what is open in it, each scrolling on its own. */
export function ListDetail({
  list,
  detail,
  width = 'list',
  side = 'start',
  className,
}: ListDetailProps) {
  const column = (
    <div className={cn('flex min-h-0 shrink-0 flex-col', width === 'list' ? 'w-66' : 'w-54')}>
      {list}
    </div>
  )
  const open = <div className="flex min-h-0 min-w-0 flex-1 flex-col">{detail}</div>

  return (
    <div data-list-detail className={cn('flex min-h-0 min-w-0 flex-1 gap-2.5', className)}>
      {side === 'start' ? column : open}
      {side === 'start' ? open : column}
    </div>
  )
}
