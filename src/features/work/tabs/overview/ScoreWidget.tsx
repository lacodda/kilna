import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { Axis, Score, Work } from '@/lib/api/types'
import { useBlindJudging } from '@/lib/blindJudging'
import { formatDay, formatTotal } from '@/lib/format'
import { trailOf } from '@/lib/overview'
import { queries } from '@/lib/query/queries'
import { labelOf, useProfile, vocabularyOf } from '@/lib/useProfile'
import { Chip } from '@/components/ui/chip'
import { Loaded } from '@/components/Loaded'
import { axisBar, MiniBars, type Bar } from '@/features/work/tabs/overview/MiniBars'
import {
  Invite,
  InviteLink,
  useLook,
  Widget,
  WidgetSkeleton,
} from '@/features/work/tabs/overview/Widget'

/** How many totals the trail under the verdict reads: the catalogue's three. */
const TRAIL = 3
/** How many axes the score carries when it has room for them. */
const AXES_BESIDE = 3

/**
 * The verdict: the widget catalogue's number - the total, its tier, and where
 * it came from (`78.0 → 86.0 → 91.0 · v10 · Sep 19`).
 *
 * Given two cells or more it carries its heaviest axes under the number, the
 * catalogue's "score with axes"; on one cell the axes are their own widget.
 * A score taken before the text was rewritten is still the score, and is
 * drawn as one that asks to be looked at again rather than hidden.
 */
export function ScoreWidget({ work }: { work: Work }) {
  const { t } = useTranslation()
  const profile = useProfile()
  const { presentation, size } = useLook()
  const { hiding } = useBlindJudging()
  const { axes, tiers } = vocabularyOf(profile.config, work.kind)

  const history = useQuery(queries.scoreHistory(work.id))
  // Whether the draft moved after it was judged: the catalogue already works
  // that out for every row, and the findings widget reads the same answer.
  const stale = useQuery({
    ...queries.catalogue(),
    select: (rows) => rows.find((row) => row.work_id === work.id)?.stale ?? false,
  })

  const to = `/works/${work.id}/score`
  const empty = history.data?.length === 0
  const aging = !hiding && stale.data === true && !empty

  return (
    <Widget
      caption={t('card.tab.score')}
      to={to}
      tone={empty ? 'empty' : aging ? 'attention' : 'plain'}
      actions={empty ? <InviteLink to={to}>{t('overview.scoreIt')}</InviteLink> : undefined}
    >
      <Loaded query={history} plain skeleton={<WidgetSkeleton />}>
        {(scores) => {
          const latest = scores[0]
          if (latest === undefined) return <Invite>{t('score.none')}</Invite>
          // The verdict the mode exists to hold back, until this card has one
          // of its own (`lib/blindJudging`).
          if (hiding) return <Invite>{t('overview.blind')}</Invite>

          const tier = latest.tier === null ? null : labelOf(tiers, latest.tier)
          const trail = trailOf(scores, TRAIL)
          const said = [
            trail.length > 1 ? trail.map((score) => formatTotal(score.total)).join(' → ') : null,
            latest.revision === null ? null : `v${latest.revision}`,
            formatDay(latest.scored_at),
          ]
            .filter((part) => part !== null)
            .join(' · ')

          const verdict = (
            <div className="flex min-w-0 flex-wrap items-baseline gap-x-2.5 gap-y-1">
              <span className="font-mono text-2xl leading-none font-semibold tabular-nums">
                {formatTotal(latest.total)}
              </span>
              {tier !== null && <Chip variant="accent">{tier}</Chip>}
              {aging && <Chip variant="warn">{t('findings.kindShort.stale-score')}</Chip>}
              {presentation !== 'card' && (
                <span className="font-mono text-xs text-faint tabular-nums">{said}</span>
              )}
            </div>
          )

          if (presentation !== 'card') return verdict

          return (
            <>
              {verdict}
              <span className="truncate font-mono text-xs text-faint tabular-nums">{said}</span>
              {size !== 's' && <MiniBars bars={heaviest(axes, latest, AXES_BESIDE)} />}
            </>
          )
        }}
      </Loaded>
    </Widget>
  )
}

/**
 * The axes a verdict leans on most, heaviest first, with their marks: what
 * the number is mostly made of. Ties keep the profile's order.
 */
function heaviest(axes: readonly Axis[], score: Score, count: number): Bar[] {
  return [...axes]
    .map((axis, index) => ({ axis, index }))
    .sort((a, b) => b.axis.weight - a.axis.weight || a.index - b.index)
    .slice(0, count)
    .map(({ axis }) => axisBar(axis, score))
}
