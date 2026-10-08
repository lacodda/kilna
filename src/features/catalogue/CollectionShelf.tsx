import { createPortal } from 'react-dom'
import { useTranslation } from 'react-i18next'
import { FolderPlus } from 'lucide-react'
import type { Collection } from '@/lib/api/types'
import { cn } from '@/lib/utils'

/** The mark of the shelf's place that makes a new collection of what is dropped. */
export const NEW_COLLECTION = ' new'

interface Props {
  collections: readonly Collection[]
  /** How many works are being carried. */
  count: number
  /** The place the pointer is over: a collection's id, or `NEW_COLLECTION`. */
  over: string | null
  /** The handlers a place wears, so it can light up under the carried rows. */
  target: (key: string) => { onPointerEnter: () => void; onPointerLeave: () => void }
}

/**
 * Where the rows being carried can be let go: a collection each, and a new
 * one - standing at the foot of the catalogue, where the bar of ticked rows
 * stands, for as long as the rows are in the air.
 *
 * At the foot rather than in a column beside the table: the collections are
 * their own screen, and a second list of them standing beside every visit
 * to the catalogue would cost the table its width for a gesture made now and
 * then. The bar's "To collection" is the same thing for the keyboard; this
 * is the pointer's way, and draws nothing until it is used.
 *
 * Each place is found by what the pointer is let go over - `data-
 * collection-drop`, as a day of the calendar carries `data-day` - not by
 * which one registered last.
 */
export function CollectionShelf({ collections, count, over, target }: Props) {
  const { t } = useTranslation()
  const place = (key: string) =>
    cn(
      'flex max-w-56 items-center gap-1.5 rounded-full border px-3 py-1 text-sm transition-colors',
      over === key ? 'border-accent bg-accent-soft text-accent' : 'border-line text-dim',
    )

  return (
    <div
      role="status"
      className="mx-auto flex w-fit max-w-full flex-wrap items-center gap-1.5 rounded-lg border border-line-2 bg-raise px-3 py-1.5 shadow-raise"
    >
      <span className="mr-1 text-sm font-semibold">{t('collections.dropInto', { count })}</span>
      {collections.map((collection) => (
        <span
          key={collection.id}
          data-collection-drop={collection.id}
          {...target(collection.id)}
          className={place(collection.id)}
        >
          <span className="min-w-0 truncate">{collection.title}</span>
          <span className="font-mono text-xs text-faint tabular-nums">
            {collection.work_ids.length}
          </span>
        </span>
      ))}
      <span
        data-collection-drop={NEW_COLLECTION}
        {...target(NEW_COLLECTION)}
        className={cn(place(NEW_COLLECTION), 'border-dashed')}
      >
        <FolderPlus aria-hidden className="size-3.5" />
        {t('collections.new')}
      </span>
    </div>
  )
}

/**
 * What follows the pointer while rows are carried: the title, or how many.
 *
 * Beside the pointer rather than under it, and not the row itself: a row is
 * the width of the table, and drawn whole it would cover the shelf it is
 * being carried to. Fixed to the viewport and out of the pointer's way, so
 * `elementFromPoint` finds the place underneath.
 */
export function CarriedRows({
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
