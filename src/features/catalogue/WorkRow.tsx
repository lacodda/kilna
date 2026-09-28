import type { ReactNode } from 'react'
import { RowContextMenu, type RowAction } from '@/components/RowMenu'
import { TableRow } from '@/components/ui/table'

interface Props {
  actions: RowAction[]
  /** Ticked: the row wears the accent, as the mockup's chosen rows do. */
  selected: boolean
  onOpen: () => void
  children: ReactNode
}

/**
 * One row of the catalogue, with both ways into its actions.
 *
 * The right click is `RowContextMenu` rendering the `<tr>` itself rather than
 * wrapping it: a `<div>` between `<tbody>` and `<tr>` is invalid markup, and
 * the browser repairs it by lifting the row out of the table.
 */
export function WorkRow({ actions, selected, onOpen, children }: Props) {
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
          className="group cursor-pointer data-[popup-open]:bg-soft"
          onClick={onOpen}
        />
      }
    >
      {children}
    </RowContextMenu>
  )
}
