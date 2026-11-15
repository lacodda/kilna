import type { HTMLAttributes } from 'react'
import { useRender } from '@base-ui/react/use-render'
import { cn } from 'dowel-ui'

/*
 * TileButton.
 *
 * A tile pressed whole: a thumbnail whose picture is the control, a card picked
 * from a row of cards. It takes the size of what it holds and draws nothing
 * inside it, so a picture keeps its well and a card keeps its own layout.
 *
 * Button cannot be this. A Button is a control row: it stands on `h-control`,
 * pads its sides, keeps its words on one line and centres them, which is what a
 * row of actions wants and what flattens a picture into a strip. kilna drew
 * scene frames and a folder's file tiles as raw buttons for that reason, and
 * style cards as a Button with its size taken away; three answers to one
 * question, each with its own focus ring or none.
 *
 * What it does decide is the edge. The tile is drawn by its border, and the
 * chosen one by the accent on that border and a second line just inside it,
 * drawn over the picture: a picture covers the inside of the tile, so a chosen
 * state painted as a fill would be under it, and one drawn outside the tile is
 * cut off by whatever scrolls a strip of them. The doubled edge is a change of
 * weight as well as hue, so the chosen tile is told apart without colour.
 *
 * `aria-current` names the chosen one, the way RowButton does: a tile that
 * opens what it shows is where the reader is, not a value being submitted. A
 * tile that toggles passes `aria-pressed`, and one in a listbox passes
 * `aria-selected`; either one replaces `aria-current` rather than joining it.
 */

export interface TileButtonProps extends HTMLAttributes<HTMLElement> {
  /** The chosen tile: the accent on its edge, and `aria-current` unless the
   * caller names the state itself with `aria-pressed` or `aria-selected`. */
  selected?: boolean
  /** The element to draw the tile as - a router's link, most often:
   * `render={<Link to={…} />}`. Without it the tile is a `<button>`. */
  render?: useRender.RenderProp
  /** Disables the tile. Applied only when it is the default `<button>`: a
   * rendered element such as a link has no `disabled` of its own. */
  disabled?: boolean
}

export function TileButton({ selected = false, render, disabled, className, ...props }: TileButtonProps) {
  // The caller's own word for the state wins, and then it is the only one: a
  // tile announced as both current and pressed says one thing twice.
  const named = props['aria-pressed'] !== undefined || props['aria-selected'] !== undefined

  return useRender({
    render,
    defaultTagName: 'button',
    props: {
      // A `<button>` inside a form submits it unless told otherwise.
      ...(render === undefined ? { type: 'button', disabled } : {}),
      'aria-current': selected && !named ? 'true' : undefined,
      'data-selected': selected ? '' : undefined,
      className: cn(
        // A column, so a picture inside is a block and sits flush at the foot
        // rather than on a line of text with room for descenders under it.
        'relative flex min-w-0 cursor-pointer flex-col overflow-hidden rounded-md border border-line bg-soft text-start text-text no-underline',
        'transition-colors not-disabled:hover:border-line-2',
        'disabled:cursor-not-allowed disabled:opacity-50',
        selected && [
          'border-accent not-disabled:hover:border-accent',
          // The second line of the chosen edge: over the picture, inside the
          // border, following its corners.
          'after:pointer-events-none after:absolute after:inset-0 after:rounded-[inherit] after:ring-1 after:ring-inset after:ring-accent',
        ],
        className,
      ),
      ...props,
    },
  })
}
