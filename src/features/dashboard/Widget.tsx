import type { ReactNode } from 'react'
import { Panel } from '@/components/ui/panel'
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
  children: ReactNode
  className?: string
}

/**
 * One block of the dashboard's side column: a caption, and what it is about.
 *
 * The mockup's `.w` - a raised panel with a small uppercase caption across its
 * top. The caption is a heading, so a screen reader can walk the column block
 * by block the way an eye does.
 */
export function Widget({ title, aside, attention = false, children, className }: Props) {
  return (
    <Panel
      className={cn(
        'flex min-w-0 shrink-0 flex-col gap-2 p-3',
        attention && 'border-warn/50',
        className,
      )}
    >
      <header className="flex min-w-0 items-center gap-2">
        <h2 className={cn('caption min-w-0 flex-1 truncate', attention && 'text-warn')}>{title}</h2>
        {aside}
      </header>
      {children}
    </Panel>
  )
}
