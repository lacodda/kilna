import {
  Archive,
  Award,
  Bookmark,
  Check,
  CircleHelp,
  Clock,
  Eye,
  Flag,
  Flame,
  Gauge,
  Gem,
  Heart,
  Layers,
  Lightbulb,
  MessageCircle,
  Mic,
  Paintbrush,
  Music,
  Pin,
  RefreshCw,
  Rocket,
  Scissors,
  ShieldAlert,
  Sparkles,
  Star,
  Tag,
  ThumbsUp,
  Video,
  Wrench,
  Zap,
  type LucideIcon,
} from 'lucide-react'
import type { Mark, MarkColour } from '@/lib/api/types'
import type { BadgeProps } from '@/components/ui/badge'

/**
 * The glyphs a mark may carry, by the name the profile writes.
 *
 * A short, closed list rather than every icon there is: the name lives in a
 * JSON document the author edits by hand, and a list they can read in the
 * profile reference beats a thousand names they cannot. A name outside it
 * draws the default glyph — the word beside it is what says which mark it is.
 */
const MARK_ICONS: Record<string, LucideIcon> = {
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
  sparkles: Sparkles,
  scissors: Scissors,
  zap: Zap,
  'shield-alert': ShieldAlert,
  'refresh-cw': RefreshCw,
  rocket: Rocket,
  award: Award,
  gem: Gem,
  music: Music,
  mic: Mic,
  video: Video,
  'message-circle': MessageCircle,
  archive: Archive,
  layers: Layers,
  paintbrush: Paintbrush,
}

/** The names a mark's glyph may take, as the profile editor offers them. */
export const MARK_ICON_NAMES = Object.keys(MARK_ICONS)

export function markIconOf(mark: Pick<Mark, 'icon'>): LucideIcon {
  return (mark.icon != null ? MARK_ICONS[mark.icon] : undefined) ?? Tag
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
