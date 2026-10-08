import type { PointerEvent, ReactNode } from 'react'
import { RowContextMenu, type RowAction } from '@/components/RowMenu'
import { TableRow } from '@/components/ui/table'
import { cn } from '@/lib/utils'

interface Props {
  actions: RowAction[]
  /** Ticked: the row wears the accent, as the mockup's chosen rows do. */
  selected: boolean
  /** Being carried: it dims where it was, as a calendar chip does. */
  carried?: boolean
  onOpen: () => void
  /** The press that may become a carry - into a collection (v0.92). */
  onGrab?: (event: PointerEvent<HTMLElement>) => void
  children: ReactNode
}

/**
 * One row of the catalogue, with both ways into its actions.
 *
 * The right click is `RowContextMenu` rendering the `<tr>` itself rather than
 * wrapping it: a `<div>` between `<tbody>` and `<tr>` is invalid markup, and
 * the browser repairs it by lifting the row out of the table.
 *
 * The row is picked up whole, not by a grip: a press is still a click that
 * opens the work until it has travelled far enough to be a carry, and the
 * text of a row of a list is not there to be selected.
 */
export function WorkRow({ actions, selected, carried = false, onOpen, onGrab, children }: Props) {
  return (
    <RowContextMenu
      actions={actions}
      render={
        <TableRow
          selected={selected}
          // `group`: the held cells paint their own ground, so the row's tint
          // cannot reach them by inheritance - they read it from here. While
          // its menu is open the row stays lit, so it is obvious which of
          // twenty rows the actions belong to.
          className={cn(
            'group cursor-pointer select-none data-[popup-open]:bg-soft',
            carried && 'opacity-50',
          )}
          onClick={onOpen}
          onPointerDown={onGrab}
        />
      }
    >
      {children}
    </RowContextMenu>
  )
}
