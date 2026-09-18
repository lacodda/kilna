import { Children, useState, type ReactNode } from 'react'
import { cn } from 'dowel-ui'
import { Button } from './button'
import { Popover, PopoverPopup, PopoverTrigger } from './popover'

/*
 * The bell in the title bar, lit by what asks to be looked at.
 *
 * A bell that is always lit is a bell nobody reads, so the count is the
 * product's decision and this draws it: nothing at zero, the number past
 * that, `9+` past nine. Pressing it does not leave the screen - the last few
 * entries open under it and the whole history is one more click, which is
 * the shape every product converged on once the first one tried a page.
 *
 * What is in the list is the product's: the rows are `children`, because a
 * journal entry, a failed upload and a comment are three different lines and
 * one component cannot draw them. What is shared is the frame around them -
 * the trigger with its badge, the heading with the button that clears the
 * count, the empty line, and the footer that leads to everything.
 *
 * Built on Popover and Button rather than on a `<div>` of its own, so the
 * panel opens, positions and closes the way every other panel does.
 */

export interface NotificationBellProps {
  /** How many things ask to be looked at. Nothing is drawn at zero. */
  count: number
  /** What the bell is called, for a screen reader and the tooltip. */
  label: string
  /** The heading of the panel. */
  title: string
  /** Open, controlled. Left out, the bell manages itself. */
  open?: boolean
  onOpenChange?: (open: boolean) => void
  /** The button that clears the count, shown only while there is one. */
  markAllLabel?: string
  onMarkAll?: () => void
  /** The footer button, which closes the panel and hands over. */
  seeAllLabel?: string
  onSeeAll?: () => void
  /** What the list says when there are no rows. */
  emptyLabel: string
  /** The rows. */
  children?: ReactNode
  /** While the product is clearing the count: the button waits. */
  busy?: boolean
  className?: string
}

export function NotificationBell({
  count,
  label,
  title,
  open,
  onOpenChange,
  markAllLabel,
  onMarkAll,
  seeAllLabel,
  onSeeAll,
  emptyLabel,
  children,
  busy = false,
  className,
}: NotificationBellProps) {
  const [own, setOwn] = useState(false)
  const isOpen = open ?? own
  const setOpen = (next: boolean) => {
    setOwn(next)
    onOpenChange?.(next)
  }
  const empty = Children.count(children) === 0

  return (
    <Popover open={isOpen} onOpenChange={setOpen}>
      <PopoverTrigger
        render={
          <Button variant="icon" size="icon-sm" className={cn('relative', className)} title={label} aria-label={label} />
        }
      >
        <Bell />
        {count > 0 ? (
          <span
            data-badge
            className={cn(
              'absolute -right-0.5 -top-0.5 min-w-3.5 rounded-full bg-warn px-1',
              'text-center font-mono text-[9px] leading-[14px] text-on-warn',
            )}
          >
            {count > 9 ? '9+' : count}
          </span>
        ) : null}
      </PopoverTrigger>
      <PopoverPopup align="end" arrow={false} size="lg" className="p-0">
        <header className="flex items-center gap-2 border-b border-line px-3 py-2">
          <h3 className="text-sm font-semibold">{title}</h3>
          {count > 0 && markAllLabel ? (
            <Button variant="ghost" size="sm" className="ml-auto" disabled={busy} onClick={onMarkAll}>
              {markAllLabel}
            </Button>
          ) : null}
        </header>
        <div className="max-h-80 overflow-y-auto px-3">
          {empty ? <p className="py-3 text-sm text-dim">{emptyLabel}</p> : children}
        </div>
        {seeAllLabel ? (
          <footer className="border-t border-line p-2">
            <Button
              variant="ghost"
              size="sm"
              className="w-full"
              onClick={() => {
                setOpen(false)
                onSeeAll?.()
              }}
            >
              {seeAllLabel}
            </Button>
          </footer>
        ) : null}
      </PopoverPopup>
    </Popover>
  )
}

function Bell() {
  return (
    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.4" aria-hidden>
      <path d="M4 11V7.5a4 4 0 0 1 8 0V11l1 1.5H3z" strokeLinejoin="round" />
      <path d="M6.5 13.5a1.5 1.5 0 0 0 3 0" strokeLinecap="round" />
    </svg>
  )
}
