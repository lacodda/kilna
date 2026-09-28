import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { ImageOff } from 'lucide-react'
import { fileSrc } from '@/lib/api/assets'
import type { StyleBrick } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'

/**
 * One brick in the dictionary: its cover, its name, the opening of what it
 * says, and how far along it is.
 *
 * The cover is the first reference. A brick is recognised by what it looks
 * like long before it is recognised by its name — that is what makes a picture
 * dictionary readable at forty entries.
 */
export function StyleBrickCard({ brick, onOpen }: { brick: StyleBrick; onOpen: () => void }) {
  const { t } = useTranslation()

  // Only for a brick that has one: a query per empty card would be a query per
  // card on a fresh dictionary.
  const references = useQuery({
    ...queries.styleReferences(brick.id),
    enabled: brick.reference_count > 0,
  })
  const cover = references.data?.[0]

  // A quiet Button in the shape of a card rather than a row of a list: the
  // cover, two lines of what it says and the count do not fit a row's slots.
  // No size, because a card is as tall as what it holds, not a control row.
  return (
    <Button
      variant="ghost"
      size={null}
      onClick={onOpen}
      className={cn(
        'items-stretch justify-start gap-3 p-2.5 text-left',
        brick.status === 'dropped' && 'opacity-55',
      )}
    >
      <div className="flex size-14 shrink-0 items-center justify-center overflow-hidden rounded-lg bg-soft">
        {cover === undefined ? (
          <ImageOff aria-hidden className="size-5 text-faint" />
        ) : (
          <img
            src={fileSrc(cover.path)}
            alt=""
            className="size-full object-cover"
            draggable={false}
          />
        )}
      </div>

      <div className="flex min-w-0 flex-col gap-0.5 font-normal whitespace-normal">
        <div className="flex min-w-0 items-center gap-1.5">
          <span className="truncate font-medium text-text">{brick.name}</span>
          {brick.status !== 'ready' && (
            <span
              className={cn(
                'shrink-0 rounded px-1.5 py-0.5 text-2xs',
                brick.status === 'draft' ? 'bg-soft text-dim' : 'bg-soft text-faint',
              )}
            >
              {t(`styles.status.${brick.status}`)}
            </span>
          )}
        </div>
        {/* The opening of the description, not the author's steer: the steer
            is bookkeeping about how the description was written. */}
        <span className="line-clamp-2 text-xs text-dim">
          {brick.description ?? t('styles.notDescribed')}
        </span>
        {brick.reference_count > 0 && (
          <span className="text-2xs text-faint tabular-nums">
            {t('styles.references', { count: brick.reference_count })}
          </span>
        )}
      </div>
    </Button>
  )
}
