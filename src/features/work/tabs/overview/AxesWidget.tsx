import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { Work } from '@/lib/api/types'
import { useBlindJudging } from '@/lib/blindJudging'
import { queries } from '@/lib/query/queries'
import { useProfile, vocabularyOf } from '@/lib/useProfile'
import { Loaded } from '@/components/Loaded'
import { axisBar, MiniBars } from '@/features/work/tabs/overview/MiniBars'
import {
  Invite,
  InviteLink,
  useLook,
  Widget,
  WidgetSkeleton,
} from '@/features/work/tabs/overview/Widget'

/**
 * The last score taken apart: every axis of the kind, its weight and its mark
 * - the widget catalogue's breakdown. Read here, judged on the Score tab.
 */
export function AxesWidget({ work }: { work: Work }) {
  const { t } = useTranslation()
  const profile = useProfile()
  const { presentation } = useLook()
  const { hiding } = useBlindJudging()
  const { axes } = vocabularyOf(profile.config, work.kind)

  const history = useQuery(queries.scoreHistory(work.id))
  const to = `/works/${work.id}/score`
  const empty = history.data?.length === 0

  return (
    <Widget
      caption={t('overview.axes')}
      to={to}
      tone={empty ? 'empty' : 'plain'}
      actions={empty ? <InviteLink to={to}>{t('overview.scoreIt')}</InviteLink> : undefined}
    >
      <Loaded query={history} plain skeleton={<WidgetSkeleton lines={4} />}>
        {(scores) => {
          const latest = scores[0]
          if (latest === undefined) {
            return <Invite>{t('overview.axesWaiting', { count: axes.length })}</Invite>
          }
          if (hiding) return <Invite>{t('overview.blind')}</Invite>

          const bars = axes.map((axis) => axisBar(axis, latest, true))
          // A row across the board has the width for the marks and not the
          // height for a bar each: they read as one line.
          if (presentation !== 'card') {
            return (
              <p className="flex flex-wrap gap-x-3 gap-y-0.5 text-sm">
                {bars.map((bar) => (
                  <span key={bar.key} className="whitespace-nowrap">
                    <span className="text-faint">{bar.label}</span>{' '}
                    <b className="font-mono font-medium tabular-nums">{bar.shown}</b>
                  </span>
                ))}
              </p>
            )
          }
          return <MiniBars bars={bars} />
        }}
      </Loaded>
    </Widget>
  )
}
