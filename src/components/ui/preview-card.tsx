import { PreviewCard as Base } from '@base-ui/react/preview-card'
import { cva, type VariantProps } from 'class-variance-authority'
import { cn } from 'dowel-ui'

/*
 * PreviewCard.
 *
 * The card that appears when a link is hovered: who the author is, what the
 * issue says, what is behind the URL. Rich content - an avatar, a few lines,
 * a figure or two - rather than the phrase a Tooltip holds.
 *
 * Positioned the same way a Popover is, through a Positioner between the
 * Portal and the Popup, so it flips and shifts to stay on screen. What differs
 * is how it opens: hovering the trigger, after a delay, and it stays open
 * while the pointer travels from the link to the card. That last part is the
 * whole trick - a card that vanishes when the pointer leaves the link cannot
 * be read, let alone clicked into.
 *
 * Same warning as Tooltip, and for the same reason. Base UI treats this as a
 * visual enhancement for sighted mouse and keyboard users: it is not reachable
 * on a touch screen and not announced by a screen reader. So NOTHING IN THE
 * CARD MAY BE THE ONLY PLACE IT APPEARS. Everything in it has to also be on
 * the page the link goes to - the card is a shortcut for people who can see
 * it, never the delivery mechanism for the information itself.
 *
 * If the content has to be reachable by everyone, this is the wrong component;
 * a Popover opened from a real button is the right one.
 */

export const previewCardPopupVariants = cva(
  [
    'rounded-lg border border-line bg-raise p-4 text-sm text-text shadow-float',
    'focus-visible:outline-none',
    // The enter and the leave. `duration-*` reads the token directly because
    // Tailwind's own utility takes a literal number.
    '[transition:opacity_var(--duration-base)_var(--ease-out),transform_var(--duration-base)_var(--ease-out)]',
    'data-[closed]:scale-[0.97] data-[closed]:opacity-0',
    'data-[starting-style]:scale-[0.97] data-[starting-style]:opacity-0',
    // Grow out of the edge it is anchored to rather than out of its own
    // middle, so the motion points back at the link.
    'origin-[var(--transform-origin)]',
  ],
  {
    variants: {
      size: {
        sm: 'w-[min(18rem,calc(100vw-2rem))]',
        md: 'w-[min(22rem,calc(100vw-2rem))]',
        lg: 'w-[min(28rem,calc(100vw-2rem))]',
      },
    },
    defaultVariants: { size: 'md' },
  },
)

/** The root. Controlled with `open` and `onOpenChange`, or left to manage
 * itself around a `PreviewCardTrigger`. */
export const PreviewCard = Base.Root

/** The link the card previews. Usually rendered as the anchor itself, so it
 * stays a real link: it navigates, it opens in a new tab, and a screen reader
 * announces it as one - which is what the card cannot do. Takes `delay`. */
export const PreviewCardTrigger = Base.Trigger

export interface PreviewCardPopupProps
  extends Base.Popup.Props,
    VariantProps<typeof previewCardPopupVariants> {
  /** Preferred side of the link. Base UI flips it when it does not fit, and
   * defaults it to the bottom. */
  side?: Base.Positioner.Props['side']
  /** Alignment along that side. Base UI centres it by default. */
  align?: Base.Positioner.Props['align']
  /** Distance from the link, in pixels. */
  sideOffset?: Base.Positioner.Props['sideOffset']
  /** Whether to draw the arrow pointing back at the link. */
  arrow?: boolean
  /** Where to portal to. Defaults to the document body, which is what keeps
   * the popup from being clipped by an ancestor. Pass an element to put it
   * somewhere else - inside an overlay that is already open, or into a
   * container being screenshotted. */
  container?: Base.Portal.Props['container']
}

/** The card. Portalled and positioned, so it is not clipped by the paragraph
 * the link sits in. */
export function PreviewCardPopup({
  size,
  side,
  align,
  sideOffset = 8,
  arrow = true,
  container,
  className,
  children,
  ...props
}: PreviewCardPopupProps) {
  return (
    <Base.Portal container={container}>
      <Base.Positioner
        side={side}
        align={align}
        sideOffset={sideOffset}
        className="[z-index:var(--z-floating)]"
      >
        <Base.Popup className={cn(previewCardPopupVariants({ size }), className)} {...props}>
          {arrow ? <PreviewCardArrow /> : null}
          {children}
        </Base.Popup>
      </Base.Positioner>
    </Base.Portal>
  )
}

/** The notch pointing back at the link. Base UI rotates it to whatever side
 * the card landed on, which is why the placement is keyed off `data-side`
 * rather than off the `side` that was asked for. */
export function PreviewCardArrow({ className, ...props }: Base.Arrow.Props) {
  return (
    <Base.Arrow
      className={cn(
        'h-2 w-2 rotate-45 border border-line bg-raise',
        'data-[side=bottom]:-top-1 data-[side=bottom]:border-r-0 data-[side=bottom]:border-b-0',
        'data-[side=top]:-bottom-1 data-[side=top]:border-t-0 data-[side=top]:border-l-0',
        'data-[side=left]:-right-1 data-[side=left]:border-b-0 data-[side=left]:border-l-0',
        'data-[side=right]:-left-1 data-[side=right]:border-r-0 data-[side=right]:border-t-0',
        className,
      )}
      {...props}
    />
  )
}
