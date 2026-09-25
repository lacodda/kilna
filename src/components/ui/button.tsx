import type { ButtonHTMLAttributes } from 'react'
import { cva, type VariantProps } from 'class-variance-authority'
import { useRender } from '@base-ui/react/use-render'
// `cn` comes from the package rather than being copied in beside the
// component (ADR 0002): a helper every primitive shares should update
// centrally, and a project installing a component already has the package for
// the theme. shadcn's own components import it from `@/lib/utils`; that is a
// per-project alias, and a copied file cannot know what it points at.
import { cn } from 'dowel-ui'

/*
 * Button.
 *
 * Five variants, because that is what the line's products actually reach for:
 * one primary action per screen, a quiet default, a soft accent for something
 * selected, a destructive one, and an icon-only.
 *
 * Every colour and every size is a token. There are no `dark:` utilities and
 * no raw values - the theme swaps underneath, so the same class list is
 * correct in both themes and in every product's accent.
 *
 * **Height is a control row, not a number.** `md` and `sm` stand on
 * `h-control` and `h-control-sm`, the rows every field of the set stands on,
 * so a button beside an input of the same size is the same height - and
 * `data-density` on a container reaches the button along with the field. They
 * used to say `h-9` and `h-7` literally, and a compact form came out with 32px
 * fields beside 36px buttons. `xs` is below the rows on purpose: it is for a
 * button inside something - a chat line, a chip, a table cell - and it grows
 * its hit area to the pointer floor rather than its box.
 *
 * **Every size says how big an icon is.** A text button used to size nothing,
 * so a lucide icon inside one drew at its own 24px - taller than the text,
 * across the whole line, and fixed at each call site by a `size-4` that the
 * next call site forgot. Each size now sizes an svg inside it, and only one
 * that has no size of its own: `[&_svg:not([class*=size-])]`. The guard is
 * not a nicety. The unguarded form the icon sizes used to have is a
 * descendant selector, one class and one element, and it outranks the single
 * class `size-5` written on the icon - so an explicit size at a call site was
 * silently overruled, and a `+` meant to be 10px drew at 14. The value is
 * unquoted (`size-` is an identifier) so the class sits in a plain string.
 */
export const buttonVariants = cva(
  [
    'inline-flex cursor-pointer items-center justify-center gap-1.5 whitespace-nowrap',
    'font-medium transition-colors',
    // Disabled is a state, not a colour: the button keeps its own hue and
    // loses contact instead, which reads the same whatever the accent is.
    'disabled:pointer-events-none disabled:opacity-50',
    '[&_svg]:shrink-0',
  ],
  {
    variants: {
      variant: {
        primary: 'rounded-md bg-accent font-semibold text-on-accent hover:bg-accent-2',
        ghost: 'rounded-md border border-line text-dim hover:border-line-2 hover:text-text',
        soft: 'rounded-md bg-accent-soft text-accent hover:bg-accent-soft/60',
        danger: 'rounded-md text-bad hover:bg-bad-soft',
        icon: 'rounded-md text-dim hover:bg-soft hover:text-text',
      },
      size: {
        xs: 'target-min h-6 gap-1 px-2 text-xs [&_svg:not([class*=size-])]:size-3',
        sm: 'h-control-sm px-2.5 text-xs [&_svg:not([class*=size-])]:size-3.5',
        md: 'h-control px-3.5 text-sm [&_svg:not([class*=size-])]:size-4',
        'icon-xs': 'target-min size-5 [&_svg:not([class*=size-])]:size-3',
        'icon-sm': 'size-7 [&_svg:not([class*=size-])]:size-3.5',
        'icon-md': 'size-8 [&_svg:not([class*=size-])]:size-4',
      },
    },
    defaultVariants: { variant: 'ghost', size: 'md' },
  },
)

export interface ButtonProps
  extends ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  /**
   * Render something else with the button's clothes on - a link, most often.
   *
   * Takes the element itself rather than a boolean: `render={<a href="…" />}`.
   * A function is also accepted, for the rare case that needs the props
   * before deciding what to build with them.
   */
  render?: useRender.RenderProp
}

export function Button({ variant, size, render, className, type, ...props }: ButtonProps) {
  return useRender({
    render,
    defaultTagName: 'button',
    props: {
      // A `<button>` inside a form submits it unless told otherwise, which
      // surprises everyone once. When rendering as something else the
      // attribute is meaningless and would land on an `<a>`, so it is only
      // set for the element that has it - `render` is what says which.
      ...(render === undefined && type === undefined ? { type: 'button' } : { type }),
      className: cn(buttonVariants({ variant, size }), className),
      ...props,
    },
  })
}
