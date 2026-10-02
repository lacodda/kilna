import type { ReactNode } from 'react'
import { Panel } from '@/components/ui/panel'
import { Scroll } from '@/components/frame'
import { cn } from '@/lib/utils'

interface Props {
  /** What the widget is about, as its caption. */
  title: ReactNode
  /** What stands at the other end of the caption: a count, a word, a link. */
  aside?: ReactNode
  /**
   * Something here is asking for the person, not only telling them: the
   * border takes the warning hue. The caption says what, so the colour is
   * never the only thing saying it.
   */
  attention?: boolean
  /**
   * The body is a list that can outgrow the column, and scrolls on its own
   * under a caption that stands: what its region is called. Without it the
   * widget is as tall as what it says - a figure, a line.
   */
  scroll?: string
  /** What stands under a scrolling body: the field that adds a line. */
  foot?: ReactNode
  children: ReactNode
  className?: string
}

/**
 * One block of the dashboard's side column: a caption, and what it is about.
 *
 * The mockup's `.w` - a raised panel with a small uppercase caption across its
 * top. The caption is a heading, so a screen reader can walk the column block
 * by block the way an eye does.
 *
 * A list (`scroll`) scrolls inside its own panel rather than taking the
 * column with it: until v0.90.1 forty findings made the column five
 * thousand pixels tall, and reading them scrolled the catalogue's figures and
 * the running tasks off the screen. The column hands each list a share of its
 * height (`DashboardView`), and the panel lays out as a grid - the caption,
 * the body in what is left, the foot - so the body's region has a height to
 * scroll against.
 */
export function Widget({
  title,
  aside,
  attention = false,
  scroll,
  foot,
  children,
  className,
}: Props) {
  return (
    <Panel
      className={cn(
        scroll === undefined
          ? 'flex min-w-0 shrink-0 flex-col gap-2 p-3'
          : 'grid min-h-0 min-w-0 grid-rows-[auto_minmax(0,1fr)] gap-2 p-3',
        attention && 'border-warn/50',
        className,
      )}
    >
      <header className="flex min-w-0 items-center gap-2">
        <h2 className={cn('caption min-w-0 flex-1 truncate', attention && 'text-warn')}>{title}</h2>
        {aside}
      </header>
      {scroll === undefined ? (
        children
      ) : (
        // Out to the panel's edges, so the bar runs down its border rather
        // than over the ends of the lines.
        <Scroll label={scroll} className="-mx-3" contentClassName="flex flex-col gap-2 px-3">
          {children}
        </Scroll>
      )}
      {foot}
    </Panel>
  )
}
