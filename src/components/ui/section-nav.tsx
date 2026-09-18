import { useId, type HTMLAttributes, type ReactNode } from 'react'
import { useRender } from '@base-ui/react/use-render'
import { cn } from 'dowel-ui'

/*
 * The left-hand list of a screen with sections, and the heading each opens.
 *
 * A settings screen is the usual case: five or six sections, each its own
 * address so it can be linked to and the back button walks between them,
 * listed down the left with the current one tinted. Every product draws the
 * same column, and every product draws the active row a little differently -
 * which is exactly the drift a shared list exists to stop.
 *
 * The rows are the product's links, not the component's. `render` takes the
 * item and answers with the element to draw it as - a router's `NavLink`, an
 * `<a>`, whatever the product navigates with - and the component puts the
 * row's clothes, its icon and its `aria-current` on it. Without `render` a
 * row is a button and `onSelect` says which one was pressed, for a screen
 * whose sections are state rather than routes.
 *
 * `SectionHeading` is the other half: the title and the one-line hint above
 * a section's body, so the column and the page it opens are set in the same
 * type.
 */

export interface SectionNavItem {
  id: string
  label: ReactNode
  icon?: ReactNode
}

export interface SectionNavProps extends Omit<HTMLAttributes<HTMLElement>, 'children' | 'onSelect'> {
  /** What the list is called: the caption above it, and the name a screen
   * reader gives the landmark. */
  label: string
  items: readonly SectionNavItem[]
  /** Which item is the current page. */
  activeId?: string
  /** The element a row is drawn as - `render={(item) => <NavLink to={…} />}`.
   * The row's props are merged onto it, the way Button takes a `render`. */
  render?: (item: SectionNavItem) => useRender.RenderProp
  /** Pressed, whatever the row is drawn as. */
  onSelect?: (id: string) => void
}

export function SectionNav({ label, items, activeId, render, onSelect, className, ...props }: SectionNavProps) {
  const captionId = useId()
  return (
    <nav aria-labelledby={captionId} className={cn('flex flex-col gap-0.5', className)} {...props}>
      <h2 id={captionId} className="px-2.5 pb-2 text-2xs font-medium uppercase tracking-caption text-faint">
        {label}
      </h2>
      {items.map((item) => (
        <SectionNavRow
          key={item.id}
          item={item}
          active={item.id === activeId}
          render={render?.(item)}
          onSelect={onSelect}
        />
      ))}
    </nav>
  )
}

function SectionNavRow({
  item,
  active,
  render,
  onSelect,
}: {
  item: SectionNavItem
  active: boolean
  render?: useRender.RenderProp
  onSelect?: (id: string) => void
}) {
  return useRender({
    render,
    defaultTagName: 'button',
    props: {
      ...(render === undefined ? { type: 'button' } : {}),
      // Named as the current page for a screen reader, which cannot see that
      // it is the tinted one.
      'aria-current': active ? 'page' : undefined,
      onClick: () => onSelect?.(item.id),
      className: cn(
        'flex w-full items-center gap-2.5 rounded-md px-2.5 py-1.5 text-left text-sm text-dim no-underline transition-colors',
        'hover:bg-soft hover:text-text [&_svg]:size-4 [&_svg]:shrink-0',
        'focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent',
        active && 'bg-accent-soft text-text [&_svg]:text-accent',
      ),
      children: (
        <>
          {item.icon ? <span aria-hidden className="contents">{item.icon}</span> : null}
          {item.label}
        </>
      ),
    },
  })
}

export interface SectionHeadingProps extends Omit<HTMLAttributes<HTMLDivElement>, 'title'> {
  title: ReactNode
  description?: ReactNode
}

/** The title of the open section and the line under it. */
export function SectionHeading({ title, description, className, ...props }: SectionHeadingProps) {
  return (
    <div className={cn('mb-4 flex flex-col gap-1', className)} {...props}>
      <h2 className="text-base font-semibold text-text">{title}</h2>
      {description ? <p className="text-sm text-dim">{description}</p> : null}
    </div>
  )
}
