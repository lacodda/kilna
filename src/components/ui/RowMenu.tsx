import { MoreHorizontal } from 'lucide-react'
import { Button } from '@/components/ui/button'
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
