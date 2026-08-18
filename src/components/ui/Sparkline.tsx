import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'

interface Props {
  /** Oldest first. Two points or more, or there is no line to draw. */
  values: number[]
  /** Highest value the scale allows, so two works can be compared by eye. */
  max?: number
  className?: string
}

const WIDTH = 120
const HEIGHT = 28
const PAD = 3

/**
 * A work's score over time, small enough to sit beside the number.
 *
 * The mockup writes the same history as `61 → 74 → 82`, which reads perfectly
 * at three scores and stops working at ten. A line holds both: the shape is
 * legible immediately, and the exact numbers are in the list underneath.
 *
 * Scaled against the axis maximum rather than against its own range, so a work
 * that moved 61 → 63 looks like the small change it was. A line normalised to
 * itself would draw that as a climb across the whole box.
 */
export function Sparkline({ values, max = 100, className }: Props) {
  const { t } = useTranslation()
  if (values.length < 2) return null

  const top = Math.max(max, ...values)
  const span = WIDTH - PAD * 2
  const rise = HEIGHT - PAD * 2

  const points = values.map((value, index) => {
    const x = PAD + (span * index) / (values.length - 1)
    // SVG y grows downward; a higher score has to sit higher.
    const y = PAD + rise - (rise * value) / top
    return [x, y] as const
  })

  const path = points.map(([x, y], index) => `${index === 0 ? 'M' : 'L'}${x.toFixed(1)} ${y.toFixed(1)}`).join(' ')
  const last = points.at(-1)!
  const first = values[0]!
  const latest = values.at(-1)!
  const direction = latest > first ? 'up' : latest < first ? 'down' : 'flat'

  return (
    <svg
      viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
      className={cn('h-7 w-[120px] shrink-0', className)}
      role="img"
      aria-label={t('score.trend', { from: first.toFixed(1), to: latest.toFixed(1) })}
    >
      <path
        d={path}
        fill="none"
        strokeWidth={1.5}
        strokeLinecap="round"
        strokeLinejoin="round"
        className={cn(
          direction === 'up' && 'stroke-good',
          direction === 'down' && 'stroke-bad',
          direction === 'flat' && 'stroke-dim',
        )}
      />
      {/* The newest point is marked: it is the one the eye is looking for. */}
      <circle
        cx={last[0]}
        cy={last[1]}
        r={2.5}
        className={cn(
          direction === 'up' && 'fill-good',
          direction === 'down' && 'fill-bad',
          direction === 'flat' && 'fill-dim',
        )}
      />
    </svg>
  )
}
