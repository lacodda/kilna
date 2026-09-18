import type { ReactNode } from 'react'
import { Funnel } from 'lucide-react'
import { cn } from 'dowel-ui'
import { Popover, PopoverPopup, PopoverTitle, PopoverTrigger } from './popover'
import { Button } from './button'

/*
 * FilterPopover.
 *
 * A funnel in a column header that opens a small panel for narrowing by
 * that column - a text box, a handful of checkboxes - with one way to clear
 * it. The shell only: what goes in the panel is the caller's, since a stage
 * is ticked and a title is typed and the popover has no opinion.
 *
 * The funnel is drawn filled while the column's filter holds something, and
 * that is the whole of the state it shows. A column with a filter on it has
 * to say so from the header, or a table narrowed by a funnel opened last
 * week looks like a table with fewer rows in it. Whether the funnel is
 * visible at rest or only on hover is left to the caller's classes: a
 * header with five funnels always showing is a header nobody can read, but
 * hiding the active one would hide the one thing that matters, so the
 * `data-active` attribute is there for a rule to key off.
 *
 * "Clear" is inside the panel rather than a second control beside the
 * funnel, because unticking three boxes one by one is the failure this
 * exists to prevent, and a panel is the place a person is already looking.
 */

export interface FilterPopoverProps {
  /** The heading inside the panel: the column's name. */
  title: string
  /** What the funnel says to assistive technology and on hover. */
  label: string
  /** Whether the column's filter holds anything; fills the funnel. */
  active: boolean
  clearLabel: string
  onClear: () => void
  /** The controls: a text box, checkboxes, whatever narrows this column. */
  children: ReactNode
  /** On the trigger, for showing it on hover or always. */
  className?: string
  align?: 'start' | 'center' | 'end'
}

export function FilterPopover({
  title,
  label,
  active,
  clearLabel,
  onClear,
  children,
  className,
  align = 'start',
}: FilterPopoverProps) {
  return (
    <Popover>
      <PopoverTrigger
        aria-label={label}
        title={label}
        data-active={active ? '' : undefined}
        className={cn(
          'inline-flex size-5 shrink-0 cursor-pointer items-center justify-center rounded-sm',
          'text-faint transition-colors hover:bg-soft hover:text-text',
          'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent',
          'data-[active]:text-accent data-[popup-open]:text-text',
          className,
        )}
      >
        <Funnel aria-hidden className={cn('size-3', active && 'fill-current')} />
      </PopoverTrigger>
      <PopoverPopup size="sm" align={align} arrow={false} className="p-3">
        <div className="flex items-center gap-2">
          {/* Sentence case rather than the header's uppercase: the panel is
              read, the header is scanned. */}
          <PopoverTitle className="normal-case tracking-normal">{title}</PopoverTitle>
          <Button
            size="sm"
            variant="ghost"
            className="ml-auto"
            disabled={!active}
            onClick={onClear}
          >
            {clearLabel}
          </Button>
        </div>
        <div className="mt-2 flex flex-col gap-1.5 text-sm normal-case tracking-normal">{children}</div>
      </PopoverPopup>
    </Popover>
  )
}
