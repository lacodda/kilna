import type { CSSProperties } from 'react'
import type { VibeGlyph } from '@/lib/api/types'
import type { ResolvedVibe } from '@/lib/vibe'
import { cn } from '@/lib/utils'

/**
 * What a work feels like, as one mark (FR-02): the colour of its mood, the
 * shape of its character, as many strokes as it is complex.
 *
 * Built of thirteen parts - five colours, five shapes, three weights - and
 * never picked from a set of pictures: a telescope, closed eyes and a pair of
 * dice cannot be told apart at fourteen pixels, and nobody remembers which
 * meant what. A wave in two strokes in a smoky blue is read by anyone who has
 * seen the three lines once.
 *
 * Drawn the way the interface's other glyphs are - a 24 grid, a stroke of
 * two, round ends - so it sits in a row of them as one of them. The strokes
 * stand far enough apart to stay three at the size a calendar's slot draws
 * them.
 *
 * The colour is the mood's own, handed over as a custom property: the
 * stylesheet reads it, and deepens it where the ground is light (`.vibe-mark`
 * in `styles.css`). The mark names no colour itself.
 */
export function VibeIcon({
  vibe,
  words,
  size = 16,
  className,
}: {
  vibe: ResolvedVibe
  /** What it says when it is pointed at, and to a screen reader. */
  words: string
  /** In pixels: 14 on a calendar's slot, 16 in a row, 20 on a card. */
  size?: number
  className?: string
}) {
  const strokes = Math.min(3, Math.max(1, vibe.complexity.strokes))
  return (
    <svg
      viewBox="0 0 24 24"
      width={size}
      height={size}
      role="img"
      aria-label={words}
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
      style={{ '--vibe-color': vibe.mood.color } as CSSProperties}
      className={cn('vibe-mark shrink-0', className)}
    >
      <title>{words}</title>
      {SHAPES[vibe.character.glyph][strokes - 1]!.map((path) => (
        <path key={path} d={path} />
      ))}
    </svg>
  )
}

/** A circle of radius `r` about the middle, as a path. */
const ring = (r: number) => `M${12 - r} 12a${r} ${r} 0 1 0 ${2 * r} 0a${r} ${r} 0 1 0 ${-2 * r} 0`
/** A diamond reaching `r` from the middle. */
const diamond = (r: number) => `M12 ${12 - r}l${r} ${r}l${-r} ${r}l${-r} ${-r}z`
/** A level wave across the mark at height `y`, its crests `rise` high. */
const wave = (y: number, rise = 3) => `M3 ${y}q2.25 ${-rise} 4.5 0t4.5 0t4.5 0t4.5 0`
/** A broken line across the mark, its points about height `y`. */
const jag = (y: number) => `M3 ${y + 1.5}l4 -3l3.5 3.5l3 -4l3.5 3.5l4 -3`
/** The same line drawn lower, for three of them in one mark. */
const lowJag = (y: number) => `M3 ${y + 1}l4 -2l3.5 2.5l3 -3l3.5 2.5l4 -2`
/** A chevron pointing on, its point at `x`. */
const chevron = (x: number) => `M${x - 5} 5l6 7l-6 7`

/**
 * Each shape drawn with one, two and three strokes. More strokes are more
 * of the same shape, never another one: a wave stays a wave.
 */
const SHAPES: Record<VibeGlyph, [string[], string[], string[]]> = {
  // A level wave: one, two, three of them, one under another. Three are
  // drawn lower and further apart (v0.169): at the fourteen pixels of a
  // calendar's slot the full crests left a third of a pixel between strokes.
  calm: [[wave(12)], [wave(8.5), wave(15.5)], [wave(5.5, 2.2), wave(12, 2.2), wave(18.5, 2.2)]],
  // A diamond, and diamonds inside it.
  playful: [[diamond(9)], [diamond(9), diamond(4.5)], [diamond(9.5), diamond(6), diamond(2.5)]],
  // A circle - even all round - and circles inside it.
  steady: [[ring(8.5)], [ring(9), ring(4.5)], [ring(9.5), ring(6), ring(2.5)]],
  // A broken line, pointed.
  // Three of them touched at any size - a stroke of two on points four
  // apart fills the six between lines - so three are drawn lower.
  edgy: [[jag(12)], [jag(8.5), jag(15.5)], [lowJag(5.5), lowJag(12), lowJag(18.5)]],
  // A chevron pointing on, and more behind it.
  drive: [[chevron(15)], [chevron(11.5), chevron(18.5)], [chevron(9), chevron(14.5), chevron(20)]],
}
