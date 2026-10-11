import { createPortal } from 'react-dom'

/**
 * What follows the pointer while rows are carried: the title, or how many -
 * from the catalogue to a collection, from the calendar's pane to a day.
 *
 * Beside the pointer rather than under it, and not the row itself: a row is
 * the width of the table, and drawn whole it would cover the shelf it is
 * being carried to. Fixed to the viewport and out of the pointer's way, so
 * `elementFromPoint` finds the place underneath.
 */
export function CarriedLabel({
  pointer,
  label,
}: {
  pointer: { x: number; y: number }
  label: string
}) {
  return createPortal(
    <div
      aria-hidden
      className="pointer-events-none fixed max-w-64 truncate rounded-full border border-accent bg-raise px-3 py-1 text-sm font-medium shadow-float [z-index:var(--z-overlay)]"
      style={{ left: pointer.x + 14, top: pointer.y + 10 }}
    >
      {label}
    </div>,
    document.body,
  )
}
