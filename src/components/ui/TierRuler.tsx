import { useTranslation } from 'react-i18next'
import type { Tier } from '@/lib/api'
import { cn } from '@/lib/utils'

interface Props {
  tiers: Tier[]
  /** Where the card stands now, on the 0-100 total. */
  score: number
  className?: string
}

/**
 * The tiers as a road, with the total standing somewhere along it.
 *
 * A tier badge says which band you are in; it does not say how far into it you
 * are, or how close the next one is - and "nearly a clip" is the thing a
 * person actually wants to know while they are still moving marks around. The
 * ruler answers both by showing the whole run at once, so the badge stops
 * being a verdict out of nowhere.
 *
 * Bands are drawn to scale, so the distances are the real ones: a tier that
 * starts at 78 sits nearly four fifths along, and the gap you are looking at
 * is the gap you have to close.
 */
export function TierRuler({ tiers, score, className }: Props) {
  const { t } = useTranslation()

  // Lowest first, and only the ones the ruler can place. A tier outside 0-100
  // is a broken profile; the validator says so, and the ruler stays readable.
  const ordered = [...tiers]
    .filter((tier) => tier.min >= 0 && tier.min <= 100)
    .sort((left, right) => left.min - right.min)

  if (ordered.length < 2) return null

  const at = Math.min(Math.max(score, 0), 100)

  return (
    <div className={cn('flex flex-col gap-1', className)}>
      <div
        role="img"
        aria-label={t('score.rulerLabel', { score: score.toFixed(1) })}
        className="relative h-1.5 w-full overflow-hidden rounded-full bg-soft"
      >
        {ordered.map((tier, index) => {
          const next = ordered[index + 1]
          const width = (next === undefined ? 100 : next.min) - tier.min
          if (width <= 0) return null

          const reached = score >= tier.min
          // The band the card is actually standing in, which is the one worth
          // picking out: a flat wash of "reached" over half the bar says only
          // that the card is not at zero.
          const standing = reached && (next === undefined || score < next.min)

          return (
            <span
              key={tier.key}
              title={`${tier.label} · ${tier.min}+`}
              style={{ left: `${tier.min}%`, width: `${width}%` }}
              className={cn(
                'absolute top-0 h-full',
                // Three states, not two: passed, standing in, still ahead.
                // Passed bands are dimmed so the current one carries the eye.
                standing ? 'bg-accent' : reached ? 'bg-accent/40' : 'bg-line-2',
                index > 0 && 'border-l border-bg',
              )}
            />
          )
        })}

        {/* Where the card stands. A dark core inside a light sheath, so the
            mark keeps its contrast over the accent band it usually sits on as
            well as over the empty road ahead. */}
        <span
          style={{ left: `${at}%` }}
          className="absolute top-1/2 h-[10px] w-[6px] -translate-x-1/2 -translate-y-1/2 rounded-full bg-bg ring-2 ring-text"
        />
      </div>

      <div className="flex justify-between text-[10px] text-faint">
        {ordered.map((tier) => (
          <span key={tier.key} className={cn(score >= tier.min && 'font-semibold text-dim')}>
            {tier.label}
          </span>
        ))}
      </div>
    </div>
  )
}
