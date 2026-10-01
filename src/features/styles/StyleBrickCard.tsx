import { useEffect, useRef } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { ImageOff, type LucideIcon } from 'lucide-react'
import { fileSrc } from '@/lib/api/assets'
import type { StyleBrick, StyleBrickStatus, StyleType } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { sampleGround, sampleStyle, slotsOf, styleName } from '@/lib/styleBrick'
import { say as sayLabel } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Chip } from '@/components/ui/chip'
// The typefaces the lettering samples are drawn with; the window fetches a
// file only when a sample needs it.
import '@/assets/fonts/samples.css'

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

/** Where a brick came from, in the tone its line is written in. */
export const ORIGIN_TONE = {
  set: 'text-faint',
  changed: 'text-warn',
  own: 'text-info',
} as const

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
 * One style of the dictionary: what it looks like across the top, its name
 * and where it stands, the opening of what it says, its type, family and
 * where it came from.
 *
 * The top leads, and it is drawn by what the brick is made of (its type's
 * `form`): three references - or its palette while it has none - for a
 * picture, a live sample of the typeface for lettering, its slots for a
 * dressing, the colour itself for a background. A style is recognised by
 * what it looks like long before it is recognised by its name.
 */
export function StyleBrickCard({ brick, type, icon: Icon, open, onOpen }: Props) {
  const { t } = useTranslation()
  const card = useRef<HTMLLIElement>(null)

  // The open style narrows the dictionary to a column; its card is kept in
  // view there, so the column says which one is open.
  useEffect(() => {
    if (open) card.current?.scrollIntoView({ block: 'nearest' })
  }, [open])

  const family = type?.families?.find((one) => one.key === brick.family)

  return (
    <li ref={card} className="flex">
      {/* A quiet Button in the shape of a card rather than a row of a list:
          the top, two lines of what it says and the chips do not fit a row's
          slots. No size, because a card is as tall as what it holds. */}
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
        <BrickTop brick={brick} type={type} />

        <span className="flex min-w-0 flex-col gap-1.5 px-3 pt-2.5 pb-3">
          <span className="flex min-w-0 items-center gap-2">
            <span className="min-w-0 flex-1 truncate text-sm font-semibold text-text">
              {styleName(brick)}
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
          <span className="flex flex-wrap items-center gap-1.5">
            <Chip>
              <Icon aria-hidden className="size-3" />
              {type === undefined ? brick.type_key : sayLabel(type.label)}
            </Chip>
            {family !== undefined && <Chip variant="soft">{sayLabel(family.label)}</Chip>}
            {brick.description === null ? (
              <Chip variant="dashed">{t('styles.notDescribed')}</Chip>
            ) : (
              brick.reference_count > 0 && (
                <Chip>{t('styles.references', { count: brick.reference_count })}</Chip>
              )
            )}
          </span>
          <span className={cn('font-mono text-xs', ORIGIN_TONE[brick.origin])}>
            {t(`styles.origin.${brick.origin}`)}
          </span>
        </span>
      </Button>
    </li>
  )
}

/** The top of a card, by what the brick is made of. */
function BrickTop({ brick, type }: { brick: StyleBrick; type: StyleType | undefined }) {
  const { t } = useTranslation()
  const form = type?.form ?? 'picture'

  // Only for a style that has any: a query per empty card would be a query
  // per card on a fresh dictionary.
  const references = useQuery({
    ...queries.styleReferences(brick.id),
    enabled: brick.reference_count > 0,
  })

  if (form === 'lettering') {
    return (
      <span
        className="flex h-18.5 items-center justify-center overflow-hidden px-2 text-2xl whitespace-nowrap"
        style={sampleGround(brick.colours)}
      >
        <span style={sampleStyle(brick.sample)}>{t('styles.sampleWord')}</span>
      </span>
    )
  }
  if (form === 'colour') {
    const colour = brick.colours[0]
    return colour === undefined ? (
      <Empty />
    ) : (
      // The colour is the subject here, not the styling: it is the brick's own.
      <span className="block h-18.5" style={{ background: colour }} />
    )
  }
  if (form === 'dressing') {
    const slots = slotsOf(brick.description ?? '')
    return (
      <span className="flex h-18.5 flex-wrap content-center items-center justify-center gap-1 overflow-hidden bg-soft px-3">
        {slots.length === 0 ? (
          <span className="text-xs text-faint">{t('styles.noSlots')}</span>
        ) : (
          slots.map((slot) => (
            <span key={slot} className="rounded-sm bg-info-soft px-1.5 font-mono text-xs text-info">
              {`{${slot}}`}
            </span>
          ))
        )}
      </span>
    )
  }

  const slots = Math.min(brick.reference_count, SHOWN)
  if (slots === 0) {
    // No pictures yet: the palette it is described by, when it has one.
    return brick.colours.length === 0 ? (
      <Empty />
    ) : (
      <span className="flex h-18.5">
        {brick.colours.map((colour, index) => (
          <span key={index} className="flex-1" style={{ background: colour }} />
        ))}
      </span>
    )
  }
  const shown = (references.data ?? []).slice(0, SHOWN)
  return (
    <span className={cn('grid h-18.5 gap-px bg-line', COLUMNS[slots])}>
      {Array.from({ length: slots }, (_, slot) => {
        const asset = shown[slot]
        // A slot waits in the ground colour while its picture loads, so the
        // strip does not reflow as they arrive.
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
      })}
    </span>
  )
}

function Empty() {
  return (
    <span className="flex h-18.5 items-center justify-center bg-soft">
      <ImageOff aria-hidden className="size-5 text-faint" />
    </span>
  )
}
