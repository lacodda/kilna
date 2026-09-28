import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { Work } from '@/lib/api/types'
import { useBlindJudging } from '@/lib/blindJudging'
import { formatDay, formatNumber, formatTotal } from '@/lib/format'
import { queries } from '@/lib/query/queries'
import { LineChart } from '@/components/ui/line-chart'
import { Sparkline } from '@/components/ui/sparkline'
import { Loaded } from '@/components/Loaded'
import {
  Invite,
  InviteLink,
  useLook,
  Widget,
  WidgetSkeleton,
} from '@/features/work/tabs/overview/Widget'

/**
 * How the total has moved, score after score: the widget catalogue's trend.
 *
 * A line rather than columns, because a score is a reading of a level that
 * was there between the readings - the work did not stop being an 86 the day
 * after it was judged. The floor is where the scores are, not zero: the
 * subject is the change, and 78 to 91 on a scale from nothing is a flat rule.
 * On a row across the board it is the sparkline beside the two ends.
 */
export function TrendWidget({ work }: { work: Work }) {
  const { t } = useTranslation()
  const { presentation } = useLook()
  const { hiding } = useBlindJudging()

  const history = useQuery(queries.scoreHistory(work.id))
  const to = `/works/${work.id}/score`
  const few = history.data !== undefined && history.data.length < 2

  return (
    <Widget
      caption={t('overview.trend')}
      to={to}
      tone={few ? 'empty' : 'plain'}
      actions={few ? <InviteLink to={to}>{t('overview.scoreIt')}</InviteLink> : undefined}
    >
      <Loaded query={history} plain skeleton={<WidgetSkeleton />}>
        {(scores) => {
          if (scores.length < 2) return <Invite>{t('overview.trendEmpty')}</Invite>
          if (hiding) return <Invite>{t('overview.blind')}</Invite>

          // The history comes newest first; a line reads left to right.
          const oldestFirst = [...scores].reverse()
          const first = oldestFirst[0]!
          const last = oldestFirst.at(-1)!
          const label = t('score.trend', {
            from: formatTotal(first.total),
            to: formatTotal(last.total),
          })

          if (presentation !== 'card') {
            return (
              <div className="flex min-w-0 items-center gap-2.5 font-mono text-xs text-faint tabular-nums">
                <span>{formatTotal(first.total)}</span>
                <Sparkline
                  values={oldestFirst.map((score) => score.total)}
                  max={100}
                  label={label}
                />
                <span className="text-text">{formatTotal(last.total)}</span>
              </div>
            )
          }

          return (
            <LineChart
              size="sm"
              tone="accent"
              ticks={3}
              label={label}
              points={oldestFirst.map((score, index) => ({ at: index, value: score.total }))}
              // The ticks land on round numbers, and say them round.
              formatTick={(value) => formatNumber(value, 0)}
              footer={
                <>
                  <span>
                    {formatTotal(first.total)} · {formatDay(first.scored_at)}
                  </span>
                  <span>
                    {formatTotal(last.total)} · {formatDay(last.scored_at)}
                  </span>
                </>
              }
            />
          )
        }}
      </Loaded>
    </Widget>
  )
}
