import type { ButtonHTMLAttributes, HTMLAttributes, Ref } from 'react'
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
 *
 * **An action over the picture stands beside the tile, not in it.** A cross
 * that takes a reference away, a menu of what to do with a frame: a button
 * inside the tile's button is a control nothing can press on its own, and a
 * press on it presses the tile too. So the two stand together on a `Tile`, and
 * `TileAction` takes a corner of it - shown while the tile is under the
 * pointer or holds the focus, always where there is no pointer to hover, and
 * on a ground of its own, because a bare glyph over a picture is lost in it.
 * kilna drew that corner three ways, each with its own ground or none.
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

export interface TileProps extends HTMLAttributes<HTMLElement> {
  /** The element to draw the tile's ground as - `render={<li />}` in a list
   * of tiles. Without it the ground is a `<div>`. */
  render?: useRender.RenderProp
}

/** What a tile and its actions stand on together: a box the actions take
 * their corners of, and the group whose hover and focus show them. A column,
 * so a TileButton inside stretches to it - when a tile has actions, size the
 * Tile rather than the button. */
export function Tile({ render, className, ...props }: TileProps) {
  return useRender({
    render,
    defaultTagName: 'div',
    props: {
      className: cn('group/tile relative flex min-w-0 flex-col', className),
      ...props,
    },
  })
}

/* The four corners, inset by the same step, so an action never sits on the
 * tile's border or under its rounded corner. Logical sides: the start is the
 * left in a left-to-right page and the right in a right-to-left one. */
const corners = {
  'top-end': 'top-1.5 end-1.5',
  'top-start': 'top-1.5 start-1.5',
  'bottom-end': 'bottom-1.5 end-1.5',
  'bottom-start': 'bottom-1.5 start-1.5',
} as const

export interface TileActionProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  /** What it does - "Remove the reference", "Frame actions". Its accessible
   * name and the word under the pointer, because it carries an icon and no
   * words. Required, and without a default, so the product can translate it. */
  label: string
  /** Which corner of the Tile it takes. */
  corner?: keyof typeof corners
  /** The element to draw it as - a menu's trigger, most often:
   * `render={<MenuTrigger />}`. It stays shown while the menu is open. */
  render?: useRender.RenderProp
  /** React 19 passes `ref` as a plain prop; declared so a caller can reach the
   * element. With `render` it reaches whatever was rendered. */
  ref?: Ref<HTMLButtonElement>
}

/** An icon button over a tile's picture, in a corner of its `Tile`. */
export function TileAction({ label, corner = 'top-end', render, ref, type, className, ...props }: TileActionProps) {
  return useRender({
    render,
    ref,
    defaultTagName: 'button',
    props: {
      // A type nobody gave is passed to nothing a `render` draws: a menu's
      // trigger sets its own, and `undefined` would overwrite it.
      ...(render === undefined ? { type: type ?? 'button' } : type === undefined ? {} : { type }),
      'aria-label': label,
      title: label,
      className: cn(
        'absolute grid size-6 cursor-pointer place-items-center rounded-md border border-line bg-raise text-dim shadow-lift',
        '[&_svg:not([class*=size-])]:size-3.5',
        'transition-[opacity,color] not-disabled:hover:text-text',
        corners[corner],
        // Hidden at rest, so a strip of tiles is a strip of pictures. Shown by
        // the tile's hover and by focus anywhere in it - a reader tabbing to
        // the tile learns there is more on it - and kept while a menu it opens
        // is open. Where there is no hover, a finger would never find it, so
        // there it is always shown.
        'opacity-0 group-hover/tile:opacity-100 group-focus-within/tile:opacity-100',
        'data-[popup-open]:opacity-100 pointer-coarse:opacity-100',
        'focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent',
        'disabled:cursor-not-allowed disabled:*:opacity-50',
        className,
      ),
      ...props,
    },
  })
}
