import {
  BookOpen,
  Camera,
  Feather,
  Film,
  Flame,
  Globe,
  Heart,
  Leaf,
  Mic,
  Moon,
  Music,
  PenTool,
  Radio,
  Sparkles,
  Star,
  Sun,
  type LucideIcon,
} from 'lucide-react'
import type { Space } from '@/lib/api/types'
import { cn } from '@/lib/utils'

/**
 * How a space looks: a colour and a mark, chosen from the sets below and kept
 * in the space's own file (ADR 0077), so a space looks the same on every
 * machine it is opened on.
 *
 * The colours are each dark enough for a white mark on them to read (4.5:1 or
 * more), which is why they are a set and not any colour: a space's swatch
 * carries its mark, and a pale one would carry it unseen.
 */
export const SPACE_COLOURS = [
  '#6E56CF',
  '#0B68CB',
  '#0D7477',
  '#18794E',
  '#8D6A00',
  '#C2410C',
  '#CE2C31',
  '#C2298A',
] as const

/** The marks a space can carry, by the name its file keeps. */
export const SPACE_MARKS: Record<string, LucideIcon> = {
  music: Music,
  mic: Mic,
  feather: Feather,
  'book-open': BookOpen,
  'pen-tool': PenTool,
  camera: Camera,
  film: Film,
  radio: Radio,
  sparkles: Sparkles,
  flame: Flame,
  leaf: Leaf,
  moon: Moon,
  sun: Sun,
  star: Star,
  heart: Heart,
  globe: Globe,
}

/** The kinds a space is made of, in the order they are offered. */
export const SPACE_KINDS = ['music', 'blog', 'novel', 'podcast'] as const

/** What a space of a kind looks like until its own colour and mark are chosen. */
const KIND_LOOK: Record<string, { colour: string; mark: string }> = {
  music: { colour: '#6E56CF', mark: 'music' },
  blog: { colour: '#0B68CB', mark: 'feather' },
  novel: { colour: '#18794E', mark: 'book-open' },
  podcast: { colour: '#C2410C', mark: 'mic' },
}

const FALLBACK = { colour: '#6E56CF', mark: 'sparkles' }

/** A kind's own look. */
export function kindLook(kind: string | null): { colour: string; mark: string } {
  return (kind !== null && KIND_LOOK[kind]) || FALLBACK
}

/** The colour and mark a space is drawn with: its own, or its kind's. */
function lookOf(space: Pick<Space, 'colour' | 'mark' | 'kind'>): {
  colour: string
  Mark: LucideIcon
} {
  const fallback = kindLook(space.kind)
  const mark = space.mark !== null && SPACE_MARKS[space.mark] ? space.mark : fallback.mark
  return {
    colour: space.colour ?? fallback.colour,
    Mark: SPACE_MARKS[mark] ?? Sparkles,
  }
}

interface GlyphProps {
  space: Pick<Space, 'colour' | 'mark' | 'kind'>
  size?: 'sm' | 'md'
  className?: string
}

/** A space's swatch: its mark, white, on its colour. */
export function SpaceGlyph({ space, size = 'sm', className }: GlyphProps) {
  const { colour, Mark } = lookOf(space)
  return (
    <span
      aria-hidden
      className={cn(
        'inline-flex shrink-0 items-center justify-center rounded-md text-white',
        size === 'sm' ? 'size-5' : 'size-8',
        className,
      )}
      style={{ backgroundColor: colour }}
    >
      <Mark className={size === 'sm' ? 'size-3' : 'size-4.5'} />
    </span>
  )
}
