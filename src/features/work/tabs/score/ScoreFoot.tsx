import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import type { Score, Tier, Work } from '@/lib/api/types'
import { formatTotal } from '@/lib/format'
import type { ToNextTier } from '@/lib/scoring'
import { say as sayLabel } from '@/lib/useProfile'
import { TierBadge, TierRuler } from '@/components/ui/tier'
import { TierPin } from '@/features/work/tabs/score/TierPin'
import { cn } from '@/lib/utils'

interface Props {
  /** The total of the marks shown above, or `null` while none are set. */
  total: number | null
  /** The tier it stands in: the one recorded, for a reading; the one the
   * marks reach, for a score being given. */
  tier: Tier | undefined
  /** Every tier of the kind, for the ruler. */
  tiers: Tier[]
  /** What the next tier costs from these marks; `undefined` at the top. */
  ahead: ToNextTier | undefined
  /** The last few recorded scores, oldest first - empty while judging blind. */
  trail: Score[]
  /** The score being read, picked out in the trail. */
  openId?: string
  /** The work, once loaded, for the pin. */
  work: Work | undefined
  /** The tier the standing score arrives at: what a pin would override. */
  scored: string | null
  /** What records the verdict, when the foot belongs to a score being given. */
  action?: ReactNode
}

/**
 * The verdict, standing at the foot of the open score: the total, its tier,
 * where it has come from, what the next tier costs - and the pin that
 * overrides it and the button that records it. The axes scroll above; this
 * never does, so the number answers every mark as it is moved.
 *
 * The advice and the ruler sat in the body under the axes until v0.80, which
 * put the sentence the tab exists to give below the fold of a six-axis card.
 */
export function ScoreFoot({
  total,
  tier,
  tiers,
  ahead,
  trail,
  openId,
  work,
  scored,
  action,
}: Props) {
  const { t } = useTranslation()

  return (
    <div className="flex w-full min-w-0 flex-col gap-2">
      {total !== null && (
        <TierRuler
          tiers={tiers.map((entry) => ({
            key: entry.key,
            label: sayLabel(entry.label),
            min: entry.min,
          }))}
          value={total}
          label={t('score.rulerLabel', { score: formatTotal(total) })}
          valueText={
            tier === undefined
              ? formatTotal(total)
              : `${formatTotal(total)}, ${sayLabel(tier.label)}`
          }
        />
      )}

      <div className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1.5">
        <span className="font-mono text-2xl font-semibold tabular-nums">
          {total === null ? '—' : formatTotal(total)}
        </span>

        {total !== null && tier !== undefined && <TierBadge label={sayLabel(tier.label)} />}

        <span className="flex min-w-0 flex-1 basis-48 flex-col gap-0.5">
          {/* The last few totals, oldest first: `78.0 → 86.0 → 91.0`. Five,
              because the trail is read at a glance, and the line beside each
              axis carries the rest. */}
          {trail.length > 1 && (
            <span
              className="truncate font-mono text-xs text-faint tabular-nums"
              title={t('score.trend', {
                from: formatTotal(trail[0]!.total),
                to: formatTotal(trail.at(-1)!.total),
              })}
            >
              {trail.map((score, index) => {
                const prior = trail[index - 1]?.total
                return (
                  <span
                    key={score.id}
                    className={cn(
                      prior !== undefined && score.total > prior && 'text-good',
                      prior !== undefined && score.total < prior && 'text-bad',
                      score.id === openId && 'font-semibold',
                    )}
                  >
                    {index > 0 && ' → '}
                    {formatTotal(score.total)}
                  </span>
                )
              })}
            </span>
          )}

          {/* The sentence the tab is for: not "you are Silver" but "you are
              four points short, and the cheapest four are here". */}
          {total !== null && (
            <span className="truncate text-xs text-dim">
              {ahead === undefined ? (
                t('score.topTier')
              ) : (
                <>
                  <b className="font-semibold text-text">
                    {t('score.toNextTier', {
                      gap: formatTotal(ahead.gap),
                      tier: sayLabel(ahead.tier.label),
                    })}
                  </b>
                  {ahead.cheapest !== undefined && (
                    <>
                      {' · '}
                      {t('score.cheapest', {
                        axis: sayLabel(ahead.cheapest.axis.label),
                        weight: ahead.cheapest.axis.weight,
                        count: ahead.cheapest.marks,
                      })}
                    </>
                  )}
                </>
              )}
            </span>
          )}
        </span>

        {/* Held by hand, or free to follow the score - beside the tier it
            overrides, and there with nothing marked: a pin is about the work,
            not about the marks on the scales. */}
        {work !== undefined && <TierPin work={work} scored={scored} />}

        {action}
      </div>
    </div>
  )
}
