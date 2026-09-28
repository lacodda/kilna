import { useEffect, useRef } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { ImageOff, type LucideIcon } from 'lucide-react'
import { fileSrc } from '@/lib/api/assets'
import type { StyleBrick, StyleBrickStatus, StyleType } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { say as sayLabel } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Chip } from '@/components/ui/chip'

/** How many references the strip across the top shows. */
const SHOWN = 3

/**
 * The status in the line's words for it: ready is done, a draft is waiting.
 *
 * `status` is a plain `string` on the backend (forward compatible with a
 * status a later build might add), so this is a lookup rather than an
 * exhaustive match — a status this build does not know draws the same tone
 * as a draft rather than failing to render.
 */
const TONE: Record<StyleBrickStatus, 'good' | 'warn' | 'soft'> = {
  ready: 'good',
  draft: 'warn',
  dropped: 'soft',
}
const toneOf = (status: string): 'good' | 'warn' | 'soft' =>
  status in TONE ? TONE[status as StyleBrickStatus] : 'warn'

/** The strip's columns, by how many pictures it holds. */
const COLUMNS = ['grid-cols-1', 'grid-cols-1', 'grid-cols-2', 'grid-cols-3'] as const

interface Props {
  brick: StyleBrick
  /** Its type as the profile names it; absent for a type the profile dropped. */
  type: StyleType | undefined
  /** Its type's glyph, from `styleIconOf`. */
  icon: LucideIcon
  /** It is the one open beside the dictionary. */
  open: boolean
  onOpen: () => void
}

/**
 * One style of the dictionary: a strip of its first references, its name and
 * where it stands, the opening of what it says, its type and its pictures.
 *
 * The pictures lead. A style is recognised by what it looks like long before
 * it is recognised by its name, which is what makes a picture dictionary
 * readable at forty entries - and three pictures say a look where one says a
 * picture. Until v0.79 the card was a row with one 56px cover.
 */
export function StyleBrickCard({ brick, type, icon: Icon, open, onOpen }: Props) {
  const { t } = useTranslation()
  const card = useRef<HTMLLIElement>(null)

  // Only for a style that has any: a query per empty card would be a query
  // per card on a fresh dictionary.
  const references = useQuery({
    ...queries.styleReferences(brick.id),
    enabled: brick.reference_count > 0,
  })
  const slots = Math.min(brick.reference_count, SHOWN)
  const shown = (references.data ?? []).slice(0, SHOWN)

  // The open style narrows the dictionary to a column; its card is kept in
  // view there, so the column says which one is open.
  useEffect(() => {
    if (open) card.current?.scrollIntoView({ block: 'nearest' })
  }, [open])

  return (
    <li ref={card} className="flex">
      {/* A quiet Button in the shape of a card rather than a row of a list:
          the pictures, two lines of what it says and the chips do not fit a
          row's slots. No size, because a card is as tall as what it holds. */}
      <Button
        variant="ghost"
        size={null}
        onClick={onOpen}
        aria-current={open ? 'true' : undefined}
        className={cn(
          'w-full flex-col items-stretch justify-start gap-0 overflow-hidden rounded-lg bg-raise p-0 text-left font-normal whitespace-normal',
          open && 'border-accent',
          brick.status === 'dropped' && 'opacity-55',
        )}
      >
        <span className={cn('grid h-18.5 gap-px bg-line', COLUMNS[slots])}>
          {slots === 0 ? (
            <span className="flex items-center justify-center bg-soft">
              <ImageOff aria-hidden className="size-5 text-faint" />
            </span>
          ) : (
            Array.from({ length: slots }, (_, slot) => {
              const asset = shown[slot]
              // A slot waits in the ground colour while its picture loads,
              // so the strip does not reflow as they arrive.
              return asset === undefined ? (
                <span key={slot} className="bg-soft" />
              ) : (
                <img
                  key={asset.id}
                  src={fileSrc(asset.path)}
                  alt=""
                  className="size-full min-w-0 object-cover"
                  draggable={false}
                />
              )
            })
          )}
        </span>

        <span className="flex min-w-0 flex-col gap-1.5 px-3 pt-2.5 pb-3">
          <span className="flex min-w-0 items-center gap-2">
            <span className="min-w-0 flex-1 truncate text-sm font-semibold text-text">
              {brick.name}
            </span>
            <Badge variant={toneOf(brick.status)} className="shrink-0">
              {t(`styles.status.${brick.status}`)}
            </Badge>
          </span>
          {/* The opening of the description, not the author's steer: the
              steer is bookkeeping about how the description was written. */}
          {brick.description !== null && (
            <span className="line-clamp-2 text-sm leading-relaxed text-dim">
              {brick.description}
            </span>
          )}
          <span className="flex flex-wrap gap-1.5">
            <Chip>
              <Icon aria-hidden className="size-3" />
              {type === undefined ? brick.type_key : sayLabel(type.label)}
            </Chip>
            {brick.description === null ? (
              <Chip variant="dashed">{t('styles.notDescribed')}</Chip>
            ) : (
              brick.reference_count > 0 && (
                <Chip>{t('styles.references', { count: brick.reference_count })}</Chip>
              )
            )}
          </span>
        </span>
      </Button>
    </li>
  )
}
