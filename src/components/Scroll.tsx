import type { ReactNode, Ref } from 'react'
import { ScrollArea } from '@/components/ui/scroll-area'
import { cn } from '@/lib/utils'

/*
 * The part of a frame that scrolls (ADR 0038).
 *
 * The frame itself - `Frame`, `Pane`, `ListDetail` - is dowel's
 * (`components/ui/frame`), harvested from the one kilna drew by hand until
 * v0.110. What dowel did not take is this: a region that scrolls inside a
 * column, with the line's overlay bar - drawn over the content, taking no
 * width, so a list that grows past the fold does not push everything beside
 * it sideways - and the two ways kilna's regions are sized, `bound` and `fit`.
 *
 * A region of widgets - the dashboard's side column, a work's overview - is a
 * `Scroll` that is `bound`: each widget in it is as tall as the region at
 * most and scrolls its own body past that (`BOUND`), so the region itself
 * scrolls only when its widgets together do not fit. A widget taller than
 * the window moved everything beside it whenever it was read (v0.90.1).
 */

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
      // A frame scrolls down, not across: its rows truncate to the column
      // instead of pushing it wider - the queue's rows ran a hundred pixels
      // past a 264px column before the axis was held.
      orientation="vertical"
      viewportRef={viewportRef}
      className={cn(!fit && 'flex-1', bound && '[container-type:size]', className)}
      viewportClassName={cn(fit && 'max-h-[inherit]')}
    >
      {/* The content's own box, inside the one Base UI measures: the
          viewport's single child is Base UI's, so a gap or a column set on
          the viewport would reach nothing. */}
      <div className={contentClassName}>{children}</div>
    </ScrollArea>
  )
}
