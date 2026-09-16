import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'
import { type Stage } from '@/lib/api'

/**
 * How finished a work is, drawn as a filled dial.
 *
 * A dial and not a badge, because the question is "how far along", and a
 * fraction answers that at a glance where a word has to be read. Sized to sit
 * beside the star: the two are the same kind of thing — something the author
 * says about the work by hand — and they belong in the same row.
 *
 * Three states, not two. `undefined` is "nobody has said yet" and draws an
 * empty ring; it is not the same as a judgement of zero, which draws a ring
 * with a dot of colour in it. A work nobody has judged should not look like a
 * work judged to be nothing.
 */
export function StageDial({
  percent,
  stage,
  size = 14,
  className,
}: {
  percent: number | null | undefined
  /** The stop this percentage belongs to, for its colour. */
  stage?: Stage
  /** Diameter in pixels. The catalogue draws it smaller than the card does. */
  size?: number
  className?: string
}) {
  const { t } = useTranslation()
  const unjudged = percent === null || percent === undefined
  const filled = unjudged ? 0 : Math.min(100, Math.max(0, percent))

  // A ring drawn with one stroke rather than wedges: `stroke-dasharray` on a
  // circle is one element and one number, where a pie of six slices is six
  // paths and an arc formula — and the ring reads better at fourteen pixels,
  // which is the size that matters here.
  const radius = 5
  const circumference = 2 * Math.PI * radius
  const tone = TONE[stage?.colour ?? 'plain']

  return (
    <svg
      viewBox="0 0 16 16"
      width={size}
      height={size}
      role="img"
      aria-label={
        unjudged
          ? t('stage.unset')
          : t('stage.atPercent', { percent: filled, stage: stage?.label ?? '' })
      }
      className={cn('shrink-0', className)}
    >
      <circle
        cx="8"
        cy="8"
        r={radius}
        fill="none"
        strokeWidth="2.5"
        className="stroke-line-2"
      />
      {!unjudged && filled > 0 && (
        <circle
          cx="8"
          cy="8"
          r={radius}
          fill="none"
          strokeWidth="2.5"
          strokeLinecap="round"
          // From twelve o'clock, clockwise: a dial that started anywhere else
          // would be read wrong by everyone who has seen a clock.
          transform="rotate(-90 8 8)"
          strokeDasharray={`${(circumference * filled) / 100} ${circumference}`}
          className={tone}
        />
      )}
      {/* A judgement of zero is still a judgement. Without this dot it would
          draw exactly like "nobody has said", and the author would have no way
          to tell that they had answered. */}
      {!unjudged && filled === 0 && <circle cx="8" cy="8" r="1.5" className={cn('fill-current', tone)} />}
    </svg>
  )
}

/** A stage's colour, by the palette role its profile named. */
const TONE: Record<string, string> = {
  plain: 'stroke-dim text-dim',
  accent: 'stroke-accent text-accent',
  good: 'stroke-good text-good',
  warn: 'stroke-warn text-warn',
  bad: 'stroke-bad text-bad',
  info: 'stroke-info text-info',
}
