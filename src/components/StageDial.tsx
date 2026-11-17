import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'
import type { Stage } from '@/lib/api/types'
import { say as sayLabel } from '@/lib/useProfile'

/**
 * How finished a work is, drawn as a dial: one of five (FR-03).
 *
 * A dial and not a badge, because the question is "how far along", and a
 * fraction answers that at a glance where a word has to be read. No word
 * stands beside one anywhere: the five are told apart by how much of the ring
 * is drawn and by colour, and the word is what the dial says when it is
 * pointed at - "25% — started / first drafts".
 *
 * - at 0, an empty grey ring and nothing in it. Until v0.119 a dot stood in
 *   its middle, to tell a judgement of zero from "nobody has said"; there is
 *   no such state any more, and a work begins here;
 * - at 25, 50 and 75, an arc of a quarter, a half and three quarters over a
 *   dim track;
 * - at 100, the whole ring closed.
 */
export function StageDial({
  stage,
  size = 14,
  className,
}: {
  /** The stop the work stands at: its place, its colour and its words. */
  stage: Stage
  /** Diameter in pixels. The catalogue draws it smaller than the card does. */
  size?: number
  className?: string
}) {
  const { t } = useTranslation()
  const filled = Math.min(100, Math.max(0, stage.percent))
  const words = stageWords(t, stage)

  // A ring drawn with one stroke rather than wedges: `stroke-dasharray` on a
  // circle is one element and one number, and the ring reads better at
  // fourteen pixels, which is the size that matters here.
  const radius = 5
  const circumference = 2 * Math.PI * radius
  const whole = filled === 0 || filled === 100

  return (
    <svg
      viewBox="0 0 16 16"
      width={size}
      height={size}
      role="img"
      aria-label={words}
      // The stop's exact colour, where the profile gives one: five stops are
      // more than the palette has roles to tell apart. Through `color`, so
      // every stroke below is `currentColor` and none names a colour itself.
      style={stage.hex === undefined ? undefined : { color: stage.hex }}
      className={cn(
        'shrink-0',
        stage.hex === undefined && TONE[stage.colour ?? 'plain'],
        className,
      )}
    >
      <title>{words}</title>
      <circle
        cx="8"
        cy="8"
        r={radius}
        fill="none"
        strokeWidth="2.5"
        stroke="currentColor"
        // The ends of the scale are the ring itself; between them it is the
        // track the arc lies on.
        strokeOpacity={whole ? 1 : 0.22}
      />
      {!whole && (
        <circle
          cx="8"
          cy="8"
          r={radius}
          fill="none"
          strokeWidth="2.5"
          strokeLinecap="round"
          stroke="currentColor"
          // From twelve o'clock, clockwise: a dial that started anywhere else
          // would be read wrong by everyone who has seen a clock.
          transform="rotate(-90 8 8)"
          strokeDasharray={`${(circumference * filled) / 100} ${circumference}`}
        />
      )}
    </svg>
  )
}

/** What a dial says when it is pointed at, and what a screen reader reads. */
export function stageWords(
  t: (key: string, values: Record<string, unknown>) => string,
  stage: Stage,
): string {
  return t('stage.atPercent', { percent: stage.percent, stage: sayLabel(stage.label) })
}

/** A stage's colour by the palette role its profile named, for a stop that
 *  gives no exact one. */
const TONE: Record<string, string> = {
  plain: 'text-dim',
  accent: 'text-accent',
  good: 'text-good',
  warn: 'text-warn',
  bad: 'text-bad',
  info: 'text-info',
}
