import { useTranslation } from 'react-i18next'
import type { Mark, MarkColour } from '@/lib/api/types'
import { markIconOf } from '@/lib/markIcon'
import { say as sayLabel } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Avatar } from '@/components/ui/avatar'
import { Tooltip, TooltipPopup, TooltipTrigger } from '@/components/ui/tooltip'

interface Props {
  /** The marks raised on a work, in the profile's order. */
  marks: readonly Mark[]
  /**
   * The ground the row stands on, for the ring each tile cuts out of the one
   * behind it: a table row and a card's header are both raised.
   */
  ring?: string
  className?: string
}

/** A mark's tile in the hue its profile named; plain wears the avatar's own. */
const GROUND: Record<MarkColour, string> = {
  plain: '',
  accent: 'bg-accent-soft text-accent',
  good: 'bg-good-soft text-good',
  warn: 'bg-warn-soft text-warn',
  bad: 'bg-bad-soft text-bad',
  info: 'bg-info-soft text-info',
}

/**
 * A work's marks as a row of small round tiles, each its icon, its name on
 * hover (v0.90.1).
 *
 * The marks were badges with their words until then - "The good one",
 * "Working on it" - and two of them took a catalogue column as wide as a
 * title. The words are the profile's and the icon already says which mark it
 * is to the person who chose it, so the row keeps the icon and the hue and
 * gives the word to the tooltip and to a screen reader: each tile is an image
 * named by its mark.
 *
 * dowel's `Avatar` drawn as its group draws it - overlapping, the first on
 * top, a ring of the ground between them - rather than `AvatarGroup` itself,
 * which takes people and their pictures and has no tile for an icon and no
 * tooltip.
 */
export function MarkAvatars({ marks, ring = 'ring-raise', className }: Props) {
  const { t } = useTranslation()
  if (marks.length === 0) return null

  return (
    <span
      role="group"
      aria-label={t('catalogue.column.marks')}
      className={cn('inline-flex items-center', className)}
    >
      {marks.map((mark, index) => {
        const Icon = markIconOf(mark)
        const name = sayLabel(mark.label)
        return (
          <Tooltip key={mark.key}>
            <TooltipTrigger
              render={
                <Avatar
                  name={name}
                  size="xs"
                  fallback={<Icon aria-hidden className="size-3" />}
                  data-mark={mark.key}
                  // The leftmost whole, the rest tucked under it - the order
                  // the eye reads the row in.
                  style={{ zIndex: marks.length - index }}
                  className={cn(
                    'ring-2',
                    ring,
                    GROUND[mark.colour ?? 'plain'],
                    index > 0 && '-ml-1.5',
                  )}
                />
              }
            />
            <TooltipPopup>{name}</TooltipPopup>
          </Tooltip>
        )
      })}
    </span>
  )
}
