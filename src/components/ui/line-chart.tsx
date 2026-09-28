import type { HTMLAttributes, ReactNode } from 'react'
import { cva, type VariantProps } from 'class-variance-authority'
import { cn } from 'dowel-ui'
import { boundsOf, pathOf, runs, ticksFor, yOf, type Bounds, type Point } from './line-scale'

/*
 * A quantity over time, where it continues between the readings.
 *
 * The distinction against its neighbours is the data's, not the drawing's. A
 * column says each period is its own sum - hours worked in a week, and there
 * is no Wednesday-afternoon figure between two weeks. A line says the value
 * existed the whole time and was sampled: an account balance, a price, a
 * temperature. Drawing a sum as a line claims readings nobody took; drawing a
 * level as columns throws away the thing being watched.
 *
 * Sparkline is the same shape with the axes taken away, for beside a figure.
 * This one has them, because a balance without a scale is a squiggle.
 *
 * **A hole breaks the line.** Interpolating across invents a reading; closing
 * the gap up moves every later point and makes the axis lie about when things
 * happened. Both are quieter than the truth, which is why the truth has to be
 * drawn deliberately.
 *
 * **The floor is not zero unless the caller says so**, and that is a departure
 * from BarChart on purpose. A bar's length is the quantity, so its baseline
 * has to be zero. A line's subject is change, and a balance between 4,900 and
 * 5,100 on a zero-based axis is a flat rule. The ticks are what keep this
 * honest: they say where the bottom is.
 *
 * The plot owns its height in pixels, for the reason BarChart does - a
 * percentage against a parent with no height of its own resolves to zero, and
 * the chart disappears without failing.
 */

export const lineChartVariants = cva('relative w-full', {
  variants: {
    size: {
      sm: '[--plot:96px]',
      md: '[--plot:160px]',
    },
    /* Room on the right for the tick labels. Drawn inside the plot they sit on
     * top of whatever the line is doing there - found by looking, with `$5,200`
     * crossed out by its own series. A gutter costs a little width and cannot
     * collide. */
    gutter: {
      true: 'pr-12',
      false: '',
    },
  },
  defaultVariants: { size: 'md', gutter: true },
})

export const lineVariants = cva('fill-none', {
  variants: {
    tone: {
      accent: 'stroke-accent',
      series: 'stroke-series-1',
      good: 'stroke-good',
      bad: 'stroke-bad',
      muted: 'stroke-dim',
    },
  },
  defaultVariants: { tone: 'accent' },
})

export interface LineChartProps
  extends Omit<HTMLAttributes<HTMLDivElement>, 'children'>,
    VariantProps<typeof lineChartVariants>,
    VariantProps<typeof lineVariants> {
  /** The readings, in order. `value: null` is a measurement not taken. */
  points: Point[]
  /** Override any edge of the plot. `min: 0` for a count, where zero is the
   * truth rather than a flattening. */
  bounds?: Partial<Bounds>
  /** What the chart as a whole says, for a reader who cannot see it. */
  label: string
  /** How many value ticks to aim for. They land on round numbers, so the count
   * is a wish rather than a promise. `0` draws none. */
  ticks?: number
  /** Turns a tick into its label. Without it the number is drawn as it is -
   * which is right for a count and wrong for money or a duration. */
  formatTick?: (value: number) => string
  /** The labels under the axis - usually the first and last reading. Two or
   * three, not one per point: the axis is not a place for a list. */
  footer?: ReactNode
}

export function LineChart({
  points,
  bounds: stated,
  label,
  ticks = 4,
  formatTick,
  footer,
  tone,
  size,
  className,
  ...props
}: LineChartProps) {
  const bounds = boundsOf(points, stated)

  /* Nothing measured is not a chart of zeroes: an empty plot with its axis is
   * the honest drawing, and the caller says so in words beside it. */
  if (bounds === null) {
    return (
      <div
        className={cn(lineChartVariants({ size, gutter: false }), className)}
        role="img"
        aria-label={label}
        {...props}
      >
        <div className="h-[var(--plot)] w-full border-b border-chart-axis" />
      </div>
    )
  }

  const drawn = runs(points, bounds)
  const rules = ticks > 0 ? ticksFor(bounds, ticks) : []

  return (
    <div
      className={cn(lineChartVariants({ size, gutter: rules.length > 0 }), className)}
      role="img"
      aria-label={label}
      {...props}
    >
      <div className="relative h-[var(--plot)] w-full border-b border-chart-axis">
        {rules.map((tick) => {
          const y = yOf(tick, bounds)
          if (y === null) return null
          return (
            <div key={tick} className="pointer-events-none absolute inset-x-0" style={{ top: `${y}%` }}>
              {/* Solid hairline: a dashed rule reads as a threshold or a
                * projection, and this is neither. */}
              <div className="h-px w-full bg-chart-grid" />
              {/* Outside the plot, in the gutter the variant reserves: a label
                * drawn over the series is a label crossed out by it. */}
              <span className="absolute -top-1.5 left-full pl-1.5 text-2xs leading-none text-faint tabular-nums">
                {formatTick ? formatTick(tick) : tick}
              </span>
            </div>
          )
        })}

        {/* `preserveAspectRatio="none"` is what lets the plot be a box of the
          * caller's shape while the maths stays in percentages: x and y scale
          * independently, which would bend a shape but cannot bend a line
          * whose every segment is straight. The stroke is drawn in absolute
          * units so it does not stretch with the box. */}
        <svg
          viewBox="0 0 100 100"
          preserveAspectRatio="none"
          className="absolute inset-0 h-full w-full overflow-visible"
          aria-hidden
        >
          {drawn.map((run) => (
            <path
              key={run[0]!.at}
              d={pathOf(run)}
              vectorEffect="non-scaling-stroke"
              strokeWidth={2}
              strokeLinecap="round"
              strokeLinejoin="round"
              className={cn(lineVariants({ tone }))}
            />
          ))}
        </svg>
      </div>

      {footer !== undefined && (
        <div className="mt-1 flex justify-between text-2xs text-faint tabular-nums">{footer}</div>
      )}
    </div>
  )
}
