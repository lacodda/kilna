import type { SVGAttributes } from 'react'
import { cva, type VariantProps } from 'class-variance-authority'
import { cn } from 'dowel-ui'

/*
 * A line small enough to sit beside the number it belongs to.
 *
 * The shape of a history, not a chart of it: no axes, no gridlines, no ticks.
 * "61 → 74 → 82" reads perfectly at three points and stops working at ten; a
 * line holds both, and the exact figures stay in the list underneath.
 *
 * Three things here are not free choices.
 *
 * **The box is drawn at the size it is shown at.** A `viewBox` wider than the
 * element scales x and y by different factors: the line bends away from the
 * data and the end dot stretches into a wedge. So the size is a variant rather
 * than a `className`, and each variant states the same numbers twice on
 * purpose - once for the geometry, once for the element - from one place. The
 * donor took a `size` object *and* a class, and every call site repeated
 * itself: a size of 52 by 16, and then a class saying the same width again.
 *
 * **The scale comes from outside.** `max` is what the axis allows, not what
 * this line happens to reach, so two works can be compared by eye. Normalised
 * to itself, a line that moved 61 → 63 would climb the whole box and read as a
 * transformation.
 *
 * **The line does not judge.** The donor coloured it green when it ended
 * higher and red when it ended lower, which is a claim the component cannot
 * support: for time-to-answer or error rate, down is the good direction. So
 * the line is drawn in the de-emphasis tone and the newest point in the accent
 * - the eye goes to "where it is now", and what that means is said in words
 * next to it, usually by a StatTile's delta. `tone` is there for a caller who
 * genuinely knows the direction's meaning.
 *
 * Not interactive, and that is the form rather than an omission: a sparkline
 * has no room for a hover target that is not the whole of it. A reader who
 * needs the exact figures gets them from the table beside it.
 */

const GEOMETRY = {
  sm: { width: 52, height: 16, dot: 1.75, stroke: 1.25 },
  md: { width: 120, height: 28, dot: 2.5, stroke: 1.5 },
} as const

export type SparklineSize = keyof typeof GEOMETRY

export const sparklineVariants = cva('shrink-0 overflow-visible', {
  variants: {
    size: {
      /* Beside a figure in a row - an axis of a rubric, a cell in a table. */
      sm: 'h-4 w-13',
      /* Beside a total, where the shape is meant to be read rather than
       * glanced at. */
      md: 'h-7 w-30',
    },
  },
  defaultVariants: { size: 'md' },
})

export const sparklineLineVariants = cva('fill-none', {
  variants: {
    tone: {
      /* The default, and the one a caller should almost always leave alone:
       * the line is context for the number beside it. */
      muted: 'stroke-dim',
      /* For a line that IS the subject - one chart on a page, nothing else to
       * defer to. */
      accent: 'stroke-accent',
      /* Only where the caller knows what the direction means. A component
       * cannot: for time-to-answer, down is the good news. */
      good: 'stroke-good',
      bad: 'stroke-bad',
    },
  },
  defaultVariants: { tone: 'muted' },
})

export const sparklineDotVariants = cva('', {
  variants: {
    tone: {
      /* The newest point carries the accent even under the muted line: it is
       * the one the eye is looking for, and one dot of colour is enough to
       * find it without the line making a claim. */
      muted: 'fill-accent',
      accent: 'fill-accent',
      good: 'fill-good',
      bad: 'fill-bad',
    },
  },
  defaultVariants: { tone: 'muted' },
})

/** Breathing room, so the stroke and the end dot are not clipped by the box. */
const PAD = 3

export interface SparklineProps
  extends Omit<SVGAttributes<SVGSVGElement>, 'values'>,
    VariantProps<typeof sparklineLineVariants> {
  /** Oldest first. Fewer than two points is not a line, and draws nothing. */
  values: number[]
  /** The top of the scale, so two lines can be compared by eye. Defaults to
   * the highest value present, which makes the line self-scaled - fine for one
   * line alone, wrong for a column of them. */
  max?: number
  /** What the line says, for a reader who cannot see it. Required: an unlabelled
   * `img` is an unlabelled image, and this one carries information. */
  label: string
  size?: SparklineSize
}

export function Sparkline({ values, max, label, size, tone, className, ...props }: SparklineProps) {
  // One point is a dot with no direction and no point; zero is nothing at all.
  if (values.length < 2) return null

  /* The default lives in `sparklineVariants` like every other variant in the
   * library; this resolves the same word for the geometry, so the numbers and
   * the classes cannot disagree about which size is being drawn. */
  const { width, height, dot, stroke } = GEOMETRY[size ?? 'md']

  /* The ceiling never sits below the data: a value above `max` would otherwise
   * be drawn outside the box. Clamped rather than rejected, because a score
   * that overshoots its stated scale is the caller's problem to notice, not a
   * reason to draw nothing. */
  const top = Math.max(...values, max ?? 0)
  /* A flat line at zero, or a flat line anywhere with no stated scale, would
   * divide by zero. It is drawn along the bottom, which is where it belongs. */
  const span = width - PAD * 2
  const rise = height - PAD * 2

  const points = values.map((value, index) => {
    const x = PAD + (span * index) / (values.length - 1)
    // SVG's y grows downward, so a larger value has to sit higher up.
    const y = top === 0 ? PAD + rise : PAD + rise - (rise * value) / top
    return [x, y] as const
  })

  const path = points
    .map(([x, y], index) => `${index === 0 ? 'M' : 'L'}${x.toFixed(1)} ${y.toFixed(1)}`)
    .join(' ')
  const [lastX, lastY] = points.at(-1)!

  return (
    <svg
      viewBox={`0 0 ${width} ${height}`}
      className={cn(sparklineVariants({ size }), className)}
      role="img"
      aria-label={label}
      {...props}
    >
      <path
        d={path}
        strokeWidth={stroke}
        strokeLinecap="round"
        strokeLinejoin="round"
        className={cn(sparklineLineVariants({ tone }))}
      />
      <circle cx={lastX} cy={lastY} r={dot} className={cn(sparklineDotVariants({ tone }))} />
    </svg>
  )
}
