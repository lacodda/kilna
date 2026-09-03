import type { ReactNode } from 'react'
import { MoreHorizontal } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  ContextMenu,
  ContextMenuItem,
  ContextMenuPopup,
  ContextMenuTrigger,
} from '@/components/ui/context-menu'
import { Menu, MenuItem, MenuPopup, MenuTrigger } from '@/components/ui/menu'

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
 * The popup, the click-outside, the Escape and the whole keyboard - arrows
 * that wrap, Home and End, type-ahead - are the registry Menu's now. What is
 * left here is the shape this app asks for: a flat list of actions behind a
 * three-dot button, which is what all four call sites want and none of them
 * should have to spell out.
 *
 * The one guard kept by hand is on the trigger. The popup is portalled to the
 * body, so choosing an action cannot reach the row underneath - but the button
 * that opens the menu is still inside that row, and the row opens the work
 * when clicked. Without this, opening the menu would also open the work.
 */
export function RowMenu({ actions, label }: { actions: RowAction[]; label: string }) {
  return (
    <Menu>
      <MenuTrigger
        render={
          <Button
            variant="icon"
            size="icon-sm"
            aria-label={label}
            title={label}
            onClick={(event) => event.stopPropagation()}
          />
        }
      >
        <MoreHorizontal aria-hidden />
      </MenuTrigger>

      <MenuPopup align="end">
        {actions.map((action) => (
          <MenuItem
            key={action.key}
            tone={action.danger === true ? 'danger' : 'default'}
            onClick={action.onSelect}
          >
            {action.label}
          </MenuItem>
        ))}
      </MenuPopup>
    </Menu>
  )
}

/**
 * The same actions, reached the other way: a right click anywhere on the row.
 *
 * It takes the identical `RowAction[]` as `RowMenu` rather than a list of its
 * own, and that is the point - a row with two ways in must not have two
 * answers. Every call site passes one array to both, so an action added in one
 * place can never be missing from the other.
 *
 * `render` makes the trigger the row itself instead of wrapping it: a `<div>`
 * inserted between `<tbody>` and `<tr>` is invalid table markup, and browsers
 * repair it by moving the row out of the table entirely.
 */
export function RowContextMenu({
  actions,
  children,
  render,
}: {
  actions: RowAction[]
  children: ReactNode
  /** The element the menu belongs to - the `<tr>`, the card, the tile. */
  render: React.ReactElement<Record<string, unknown>>
}) {
  return (
    <ContextMenu>
      <ContextMenuTrigger render={render}>{children}</ContextMenuTrigger>

      <ContextMenuPopup>
        {actions.map((action) => (
          <ContextMenuItem
            key={action.key}
            tone={action.danger === true ? 'danger' : 'default'}
            onClick={action.onSelect}
          >
            {action.label}
          </ContextMenuItem>
        ))}
      </ContextMenuPopup>
    </ContextMenu>
  )
}
