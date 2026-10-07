import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { Work } from '@/lib/api/types'
import { formatDay, formatMoment } from '@/lib/format'
import { queries } from '@/lib/query/queries'
import { axisOf, hasTime, MOMENT_KINDS } from '@/lib/timeline'
import { Loaded } from '@/components/Loaded'
import { useSayMoment } from '@/features/work/tabs/history/moments'
import { Invite, useLook, Widget, WidgetSkeleton } from '@/features/work/tabs/overview/Widget'

/** How many lines the widget reads before handing over to the History tab. */
const LINES = { s: 3, m: 5, l: 8 } as const

const EVERYTHING = new Set(MOMENT_KINDS)

/**
 * The last few things that happened to the work, newest first - the same
 * moments as its History tab, where the widget goes, without what is still
 * ahead: recent is what has happened.
 */
export function RecentWidget({ work }: { work: Work }) {
  const { t } = useTranslation()
  const { size } = useLook()
  const moments = useQuery(queries.timeline(work.id))
  const say = useSayMoment(work.id)

  return (
    <Widget caption={t('overview.recent')} to={`/works/${work.id}/history`} go={t('journal.open')}>
      <Loaded query={moments} plain skeleton={<WidgetSkeleton />}>
        {(all) => {
          const past = axisOf(all, EVERYTHING).days.flatMap((day) => day.moments)
          // Nothing to invite: history is what happens, not what is made.
          return past.length === 0 ? (
            <Invite>{t('empty.historyTitle')}</Invite>
          ) : (
            <ul className="flex flex-col">
              {past.slice(0, LINES[size]).map((moment, index) => {
                const said = say(moment)
                return (
                  <li
                    key={`${moment.type}-${moment.at}-${index}`}
                    className="flex min-w-0 items-baseline gap-2 border-b border-line py-1 text-sm last:border-b-0"
                  >
                    <span className="min-w-0 flex-1 truncate" title={said.text}>
                      {said.text}
                    </span>
                    <time
                      dateTime={moment.at}
                      className="shrink-0 font-mono text-xs text-faint tabular-nums"
                    >
                      {hasTime(moment.at) ? formatMoment(moment.at) : formatDay(moment.at)}
                    </time>
                  </li>
                )
              })}
            </ul>
          )
        }}
      </Loaded>
    </Widget>
  )
}
