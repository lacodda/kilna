import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { Work } from '@/lib/api/types'
import { formatMoment } from '@/lib/format'
import { queries } from '@/lib/query/queries'
import { Loaded } from '@/components/Loaded'
import { sentence } from '@/features/journal/JournalFeed'
import { Invite, useLook, Widget, WidgetSkeleton } from '@/features/work/tabs/overview/Widget'

/** How many lines the widget reads before handing over to the History tab. */
const LINES = { s: 3, m: 5, l: 8 } as const

/**
 * The last few things that happened to the work, newest first - the same
 * lines as its History tab, where the widget goes.
 */
export function RecentWidget({ work }: { work: Work }) {
  const { t } = useTranslation()
  const { size } = useLook()
  const entries = useQuery(queries.journalForWork(work.id))

  return (
    <Widget caption={t('overview.recent')} to={`/works/${work.id}/history`} go={t('journal.open')}>
      <Loaded query={entries} plain skeleton={<WidgetSkeleton />}>
        {(lines) =>
          // Nothing to invite: history is what happens, not what is made.
          lines.length === 0 ? (
            <Invite>{t('empty.historyTitle')}</Invite>
          ) : (
            <ul className="flex flex-col">
              {lines.slice(0, LINES[size]).map((entry) => {
                const said = sentence(entry, t)
                return (
                  <li
                    key={entry.id}
                    className="flex min-w-0 items-baseline gap-2 border-b border-line py-1 text-sm last:border-b-0"
                  >
                    <span className="min-w-0 flex-1 truncate" title={said}>
                      {said}
                    </span>
                    <time
                      dateTime={entry.created_at}
                      className="shrink-0 font-mono text-xs text-faint tabular-nums"
                    >
                      {formatMoment(entry.created_at)}
                    </time>
                  </li>
                )
              })}
            </ul>
          )
        }
      </Loaded>
    </Widget>
  )
}
