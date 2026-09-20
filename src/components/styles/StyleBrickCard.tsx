import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { ImageOff } from 'lucide-react'
import { fileSrc, styleBrickReferences, type StyleBrick } from '@/lib/api'
import { keys } from '@/lib/query'
import { cn } from '@/lib/utils'

/**
 * One brick in the dictionary: its cover, its name, the opening of what it
 * says, and how far along it is.
 *
 * The cover is the first reference. A brick is recognised by what it looks
 * like long before it is recognised by its name — that is what makes a picture
 * dictionary readable at forty entries.
 */
export function StyleBrickCard({
  brick,
  onOpen,
}: {
  brick: StyleBrick
  onOpen: () => void
}) {
  const { t } = useTranslation()

  // Only for a brick that has one: a query per empty card would be a query per
  // card on a fresh dictionary.
  const references = useQuery({
    queryKey: keys.styleReferences(brick.id),
    queryFn: () => styleBrickReferences(brick.id),
    enabled: brick.reference_count > 0,
  })
  const cover = references.data?.[0]

  return (
    <button
      type="button"
      onClick={onOpen}
      className={cn(
        'flex cursor-pointer gap-3 rounded-xl border border-line bg-raise p-2.5 text-left transition-colors',
        'hover:border-line-2',
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

      <div className="flex min-w-0 flex-col gap-0.5">
        <div className="flex min-w-0 items-center gap-1.5">
          <span className="truncate font-medium">{brick.name}</span>
          {brick.status !== 'ready' && (
            <span
              className={cn(
                'shrink-0 rounded px-1.5 py-0.5 text-[10.5px]',
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
          <span className="text-[10.5px] text-faint tabular-nums">
            {t('styles.references', { count: brick.reference_count })}
          </span>
        )}
      </div>
    </button>
  )
}
