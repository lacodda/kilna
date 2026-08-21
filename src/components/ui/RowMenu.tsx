import { useEffect, useId, useRef, useState } from 'react'
import { MoreHorizontal } from 'lucide-react'
import { cn } from '@/lib/utils'

export interface RowAction {
  key: string
  label: string
  onSelect: () => void
  /** Draws it apart from the rest, in the colour of something you cannot undo. */
  danger?: boolean
}

/**
 * The menu on a row.
 *
 * Built here rather than pulled in: it is a list of buttons in a popover, and
 * the kit already carries Radix for the two places that genuinely need focus
 * management. What this does need is to not swallow the row underneath — every
 * click inside it stops there, or opening the menu would also open the work.
 */
export function RowMenu({ actions, label }: { actions: RowAction[]; label: string }) {
  const [open, setOpen] = useState(false)
  const container = useRef<HTMLDivElement>(null)
  const menuId = useId()

  useEffect(() => {
    if (!open) return

    const onPointerDown = (event: PointerEvent) => {
      if (!container.current?.contains(event.target as Node)) setOpen(false)
    }
    // Escape closes from anywhere, including from a focused item inside.
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setOpen(false)
    }

    document.addEventListener('pointerdown', onPointerDown)
    document.addEventListener('keydown', onKey)
    return () => {
      document.removeEventListener('pointerdown', onPointerDown)
      document.removeEventListener('keydown', onKey)
    }
  }, [open])

  return (
    <div
      ref={container}
      className="relative"
      // The row this sits in opens the work when clicked. Everything in here is
      // about the row rather than a way into it.
      onClick={(event) => event.stopPropagation()}
    >
      <button
        type="button"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-controls={open ? menuId : undefined}
        aria-label={label}
        title={label}
        onClick={() => setOpen((current) => !current)}
        className={cn(
          'inline-flex size-7 cursor-pointer items-center justify-center rounded-[9px] text-faint transition-colors hover:bg-soft hover:text-text',
          open && 'bg-soft text-text',
        )}
      >
        <MoreHorizontal aria-hidden className="size-4" />
      </button>

      {open && (
        <div
          id={menuId}
          role="menu"
          className="absolute right-0 z-30 mt-1 min-w-44 rounded-[10px] border border-line bg-raise p-1 shadow-lg"
        >
          {actions.map((action) => (
            <button
              key={action.key}
              type="button"
              role="menuitem"
              onClick={() => {
                setOpen(false)
                action.onSelect()
              }}
              className={cn(
                'flex w-full cursor-pointer items-center rounded-[7px] px-2.5 py-1.5 text-left text-[13px] transition-colors',
                action.danger ? 'text-bad hover:bg-bad-soft' : 'text-dim hover:bg-soft hover:text-text',
              )}
            >
              {action.label}
            </button>
          ))}
        </div>
      )}
    </div>
  )
}
