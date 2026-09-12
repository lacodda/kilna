import {
  Bookmark,
  Check,
  CircleHelp,
  Clock,
  Eye,
  Flag,
  Flame,
  Gauge,
  Heart,
  Lightbulb,
  Pin,
  Star,
  Tag,
  ThumbsUp,
  Wrench,
  type LucideIcon,
} from 'lucide-react'
import type { Mark, MarkColour } from '@/lib/api'
import type { BadgeProps } from '@/components/ui/badge'

/**
 * The glyphs a mark may carry, by the name the profile writes.
 *
 * A short, closed list rather than every icon there is: the name lives in a
 * JSON document the author edits by hand, and a list they can read in the
 * profile reference beats a thousand names they cannot. A name outside it
 * draws the default glyph — the word beside it is what says which mark it is.
 */
export const MARK_ICONS: Record<string, LucideIcon> = {
  tag: Tag,
  wrench: Wrench,
  clock: Clock,
  'circle-help': CircleHelp,
  flame: Flame,
  star: Star,
  bookmark: Bookmark,
  eye: Eye,
  check: Check,
  heart: Heart,
  lightbulb: Lightbulb,
  'thumbs-up': ThumbsUp,
  gauge: Gauge,
  flag: Flag,
  pin: Pin,
}

/** The names, for the reference and for a picker. */
export const MARK_ICON_NAMES = Object.keys(MARK_ICONS)

export function markIconOf(mark: Pick<Mark, 'icon'>): LucideIcon {
  return (mark.icon !== undefined ? MARK_ICONS[mark.icon] : undefined) ?? Tag
}

/**
 * The badge variant a palette role names.
 *
 * `plain` is the soft grey — a mark raised, a status set aside — and no colour
 * at all is the outline: the quiet reading for a status that is only a word.
 */
export function badgeVariantOf(colour: MarkColour | undefined | null): BadgeProps['variant'] {
  switch (colour) {
    case 'plain':
      return 'soft'
    case 'accent':
    case 'good':
    case 'warn':
    case 'bad':
    case 'info':
      return colour
    default:
      return 'outline'
  }
}
