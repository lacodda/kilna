import type { ReactNode } from 'react'
import { Combobox as Base } from '@base-ui/react/combobox'
import { cva, type VariantProps } from 'class-variance-authority'
import { cn } from 'dowel-ui'
import { comboboxItemVariants } from './combobox'
import { Kbd } from './kbd'

/*
 * CommandPalette.
 *
 * One box that finds anything: the shortcut opens it, typing narrows a list,
 * Enter runs what is highlighted.
 *
 * It is a Combobox rather than a Dialog with a field in it, and that is Base
 * UI's own arrangement rather than a shortcut taken here: put the input
 * *inside* the popup and the popup becomes `role="dialog"` on its own, with
 * the input still announced as the combobox that owns the list. The filtering,
 * the highlight, the arrow keys and the type-ahead are the ones Combobox
 * already has - there is no second implementation of any of it.
 *
 * What is left for the product is everything that makes a palette that
 * product's: what the items are, how they are grouped, what running one does.
 * `items` is deliberately `unknown[]` - a palette lists commands, works,
 * settings and recent files in the same box, and a type that admitted only
 * strings would push every product into the same stringly-typed workaround.
 */

export const commandPalettePopupVariants = cva(
  [
    'flex w-[min(36rem,calc(100vw-2rem))] flex-col overflow-hidden',
    'rounded-xl border border-line bg-raise text-text shadow-float',
    'focus-visible:outline-none',
    '[transition:opacity_var(--duration-quick)_var(--ease-out),transform_var(--duration-quick)_var(--ease-out)]',
    'data-[closed]:scale-[0.98] data-[closed]:opacity-0',
    'data-[starting-style]:scale-[0.98] data-[starting-style]:opacity-0',
  ],
  {
    variants: {
      size: {
        md: 'max-h-[24rem]',
        lg: 'max-h-[32rem]',
      },
    },
    defaultVariants: { size: 'md' },
  },
)

/** The root. Controlled by `open`/`onOpenChange`, because what opens a palette
 * is a shortcut somewhere else in the application. */
export const CommandPalette = Base.Root

/** A row. The same clothes as a Combobox row, on purpose: a palette is a list
 * of choices, and two lists of choices in one product should not differ. */
export const CommandPaletteItem = Base.Item

/** The list. Takes a render function over the filtered items. */
export const CommandPaletteList = Base.List

/** Shown when nothing matches. The words are the product's. */
export const CommandPaletteEmpty = Base.Empty

/** A labelled group, for a palette that lists more than one kind of thing. */
export const CommandPaletteGroup = Base.Group

export interface CommandPalettePopupProps
  extends Omit<Base.Popup.Props, 'aria-label'>,
    VariantProps<typeof commandPalettePopupVariants> {
  /**
   * What the palette is called, for a screen reader. Required, and required
   * for a reason particular to this component: the popup is a dialog, and a
   * dialog is named by its own visible title - which a palette does not have,
   * because the field is the first thing in it.
   *
   * So the name has to come from outside, it has to be the product's word, and
   * nothing else can supply it. A palette without one is announced as "dialog"
   * and nothing more; making the prop required is what stops that shipping.
   */
  'aria-label': string
  /** Where to portal to. Defaults to the document body. */
  container?: Base.Portal.Props['container']
}

/** The palette itself: a dim over the page, and the box in the upper third of
 * it - where the eye already is, rather than dead centre. */
export function CommandPalettePopup({
  size,
  container,
  className,
  children,
  ...props
}: CommandPalettePopupProps) {
  return (
    <Base.Portal container={container}>
      <Base.Backdrop
        className={cn(
          'fixed inset-0 bg-black/55 backdrop-blur-[2px]',
          '[z-index:var(--z-overlay)]',
          '[transition:opacity_var(--duration-quick)_var(--ease-out)]',
          'data-[closed]:opacity-0 data-[starting-style]:opacity-0',
        )}
      />
      <Base.Positioner
        className="[z-index:var(--z-palette)]"
        // Anchored to the viewport rather than to a trigger: a palette has no
        // trigger to point at, and the one it does have is a keystroke.
        positionMethod="fixed"
        side="bottom"
        align="center"
        sideOffset={0}
        alignOffset={0}
      >
        <Base.Popup
          className={cn(commandPalettePopupVariants({ size }), className)}
          {...props}
        >
          {children}
        </Base.Popup>
      </Base.Positioner>
    </Base.Portal>
  )
}

export interface CommandPaletteInputProps extends Base.Input.Props {
  /** Shown at the right of the field, as `['Esc']`. Decorative. */
  hint?: string[]
}

/** The field. Sits inside the popup, which is what makes the popup a dialog
 * and the field its combobox. */
export function CommandPaletteInput({ hint, className, ...props }: CommandPaletteInputProps) {
  return (
    <div className="flex items-center gap-2 border-b border-line px-3">
      <MagnifierIcon />
      <Base.Input
        className={cn(
          'h-11 w-full bg-transparent text-sm text-text placeholder:text-faint',
          'focus-visible:outline-none',
          className,
        )}
        {...props}
      />
      {hint && <Kbd keys={hint} aria-hidden className="shrink-0" />}
    </div>
  )
}

/** The row's own layout: an icon, what it is, and where it lives. */
export function CommandPaletteRow({
  icon,
  hint,
  className,
  children,
  ...props
}: {
  icon?: ReactNode
  hint?: ReactNode
  className?: string
  children: ReactNode
} & Omit<React.HTMLAttributes<HTMLDivElement>, 'children'>) {
  return (
    <div className={cn('flex w-full items-center gap-2.5', className)} {...props}>
      {icon}
      <span className="min-w-0 flex-1 truncate">{children}</span>
      {hint !== undefined && <span className="shrink-0 text-2xs text-faint">{hint}</span>}
    </div>
  )
}

export { comboboxItemVariants as commandPaletteItemVariants }

function MagnifierIcon() {
  return (
    <svg
      viewBox="0 0 16 16"
      width="15"
      height="15"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      aria-hidden
      className="shrink-0 text-faint"
    >
      <circle cx="7" cy="7" r="4.5" />
      <path d="M10.5 10.5L14 14" strokeLinecap="round" />
    </svg>
  )
}
