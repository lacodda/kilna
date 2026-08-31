import type { ButtonHTMLAttributes } from 'react'
import { Slot } from '@radix-ui/react-slot'
import { cva, type VariantProps } from 'class-variance-authority'
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
        sm: 'h-7 px-2.5 text-xs',
        md: 'h-9 px-3.5 text-sm',
        'icon-sm': 'size-7 [&_svg]:size-3.5',
        'icon-md': 'size-8 [&_svg]:size-4',
      },
    },
    defaultVariants: { variant: 'ghost', size: 'md' },
  },
)

export interface ButtonProps
  extends ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  /** Render as the child element instead of a `<button>`, keeping the styling.
   * For a link that should look like a button. */
  asChild?: boolean
}

export function Button({ variant, size, asChild = false, className, type, ...props }: ButtonProps) {
  const Component = asChild ? Slot : 'button'
  return (
    <Component
      // A `<button>` inside a form submits it unless told otherwise, which
      // surprises everyone once. When rendering as something else, the type
      // belongs to the caller.
      type={asChild ? type : (type ?? 'button')}
      className={cn(buttonVariants({ variant, size }), className)}
      {...props}
    />
  )
}
