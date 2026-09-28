import type { HTMLAttributes, ReactNode } from 'react'
import { cva, type VariantProps } from 'class-variance-authority'
import { cn } from 'dowel-ui'

/*
 * Divider.
 *
 * The line between one part of a screen and the next. The set already had
 * four of them - in the menu, the select, the context menu and the action bar
 * - and each was written inside the thing it divided, so a screen that wanted
 * a rule between two sections had nothing and reached for a bare `<hr>` or a
 * `div` with a background.
 *
 * Those two are not the same as this one, and the difference is the reason
 * this component is not a `<div className="h-px bg-line" />`. A rule between
 * sections is a semantic boundary: `role="separator"` is what tells a reader
 * that the content after it is a different thing, and a `div` tells them
 * nothing. A rule that is only decoration - a hairline inside a card, a tick
 * between two numbers - is the opposite case, and `decorative` takes it back
 * out of the accessibility tree, because a reader announcing "separator" six
 * times in one row is being read the styling.
 *
 * With a `label`, the line becomes a captioned break: the rule runs to the
 * caption, the caption sits in it, and the rule continues. It is a heading
 * for a run of content that does not deserve a heading - "Today", "Archived",
 * "or".
 *
 * A `div` of its own rather than Base UI's Separator, which was the obvious
 * dependency and does not do the one thing this is for: it renders a `div`
 * with `aria-orientation` and no `role`, so nothing is announced as a
 * separator at all - measured, not assumed, by rendering it. And there is no
 * way to take the role back off for a decorative rule, because there was
 * never one on. Two elements and two attributes are not worth borrowing
 * anyway; the decision here is which attributes, and that is the part a
 * dependency was not making.
 */

export const dividerVariants = cva('shrink-0 bg-line', {
  variants: {
    orientation: {
      horizontal: 'h-px w-full',
      /* `self-stretch` rather than `h-full`: a vertical rule in a flex row of
       * buttons has no height of its own to be a percentage of, and `h-full`
       * resolved to zero - a rule that was there in the markup and invisible
       * on the screen. */
      vertical: 'w-px self-stretch',
    },
    /* How much air it has. `none` is for a rule that is already inside
     * something padded - a list's rows, a table. */
    spacing: {
      none: '',
      sm: '',
      md: '',
      lg: '',
    },
  },
  compoundVariants: [
    { orientation: 'horizontal', spacing: 'sm', className: 'my-2' },
    { orientation: 'horizontal', spacing: 'md', className: 'my-4' },
    { orientation: 'horizontal', spacing: 'lg', className: 'my-8' },
    { orientation: 'vertical', spacing: 'sm', className: 'mx-2' },
    { orientation: 'vertical', spacing: 'md', className: 'mx-4' },
    { orientation: 'vertical', spacing: 'lg', className: 'mx-8' },
  ],
  defaultVariants: { orientation: 'horizontal', spacing: 'none' },
})

export interface DividerProps
  extends Omit<HTMLAttributes<HTMLDivElement>, 'children'>,
    Omit<VariantProps<typeof dividerVariants>, 'orientation'> {
  /** Declared here rather than taken from the variants: cva types a variant
   * as nullable, and `aria-orientation` is not - a `null` passed through
   * would be a type error at the call site and an attribute React drops. */
  orientation?: 'horizontal' | 'vertical'
  /** A caption sitting in the line: "Today", "Archived", "or". Horizontal
   * only - there is no reading direction that puts a word inside a vertical
   * rule. */
  label?: ReactNode
  /** A rule that is styling rather than structure. Taken out of the
   * accessibility tree, so a reader is not told about it. */
  decorative?: boolean
}

export function Divider({
  orientation = 'horizontal',
  spacing,
  label,
  decorative = false,
  className,
  ...props
}: DividerProps) {
  if (label !== undefined && orientation === 'horizontal') {
    return (
      <div
        // The row is what carries the separator's meaning; the two rules
        // inside it are drawing. Without `aria-hidden` on them a reader hears
        // the break announced twice with a word in the middle.
        role={decorative ? 'presentation' : 'separator'}
        aria-orientation={decorative ? undefined : 'horizontal'}
        className={cn(
          'flex w-full items-center gap-3',
          dividerVariants({ orientation, spacing }),
          // The row is not itself the line: it holds two. The variant is
          // still asked for its spacing, so a captioned break sits in the
          // same rhythm as a plain one.
          'h-auto bg-transparent',
          className,
        )}
        {...props}
      >
        <span aria-hidden className="h-px flex-1 bg-line" />
        <span className="caption shrink-0">
          {label}
        </span>
        <span aria-hidden className="h-px flex-1 bg-line" />
      </div>
    )
  }

  return (
    <div
      // `presentation` rather than no role at all: an element with no role is
      // still a `div`, which a reader may announce as a group when it carries
      // other attributes. This says outright that there is nothing here.
      role={decorative ? 'presentation' : 'separator'}
      // Only on the one that has a role to orient. A presentational element
      // carrying an aria attribute is itself a violation - the attribute
      // describes a role the element has just disclaimed.
      aria-orientation={decorative ? undefined : orientation}
      className={cn(dividerVariants({ orientation, spacing }), className)}
      {...props}
    />
  )
}

export interface SectionHeaderProps extends Omit<HTMLAttributes<HTMLDivElement>, 'title'> {
  title: ReactNode
  description?: ReactNode
  /** Buttons belonging to this section rather than to the screen - Add,
   * Collapse, a link out. */
  actions?: ReactNode
}

/**
 * The heading over a part of a screen, with what belongs to that part at the
 * far end of the same row.
 *
 * An `h2`: `PageHeader` holds the screen's `h1`, and a section is one level
 * inside it, so a reader moving by heading walks the screen's actual
 * structure. It is the between-size heading the products kept writing by
 * hand, above `SectionLabel`'s uppercase caption and below the page's title.
 */
export function SectionHeader({
  title,
  description,
  actions,
  className,
  ...props
}: SectionHeaderProps) {
  return (
    <div
      className={cn('mb-3 flex flex-wrap items-start justify-between gap-x-4 gap-y-1', className)}
      {...props}
    >
      <div className="flex min-w-0 flex-col gap-0.5">
        <h2 className="text-base font-semibold text-text">{title}</h2>
        {description === undefined ? null : <p className="text-sm text-dim">{description}</p>}
      </div>
      {actions === undefined ? null : (
        <div className="flex shrink-0 items-center gap-2">{actions}</div>
      )}
    </div>
  )
}
