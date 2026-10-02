import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { useNavigate } from 'react-router'
import { formatMoment } from '@/lib/format'
import { queries } from '@/lib/query/queries'
import { Button } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/skeleton'
import { Loaded } from '@/components/Loaded'
import { sentence } from '@/features/journal/JournalFeed'
import { Widget } from '@/features/dashboard/Widget'

/** How many new lines the widget shows before it hands over to the history. */
const SINCE_LIMIT = 6

/**
 * What happened since the person last looked, newest first.
 *
 * "Last looked" is the journal's own mark: an entry is unread until the
 * history, or the bell, is marked seen - the one moment the app knows the
 * person caught up. Read from the feed the history screen already fetches, so
 * this is a filter and not a question of its own; it marks nothing read,
 * because glancing at the dashboard is not reading the history.
 */
export function SinceWidget() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const entries = useQuery(queries.journalFeed())

  return (
    <Widget
      title={t('dashboard.since')}
      scroll={t('dashboard.since')}
      aside={
        <span className="text-xs">
          <Button variant="link" onClick={() => navigate('/journal')}>
            {t('journal.seeAll')}
          </Button>
        </span>
      }
    >
      <Loaded query={entries} plain skeleton={<Skeleton className="h-4 w-full" />}>
        {(data) => {
          const unread = data.filter((entry) => entry.read_at === null)
          if (unread.length === 0) {
            return <p className="text-sm text-faint">{t('dashboard.sinceNothing')}</p>
          }
          const more = unread.length - SINCE_LIMIT
          return (
            <>
              <ul className="flex flex-col">
                {unread.slice(0, SINCE_LIMIT).map((entry) => (
                  <li
                    key={entry.id}
                    className="flex min-w-0 items-baseline gap-2 border-b border-line py-1.5 text-sm first:pt-0 last:border-b-0"
                  >
                    <span className="min-w-0 flex-1 truncate" title={sentence(entry, t)}>
                      {sentence(entry, t)}
                    </span>
                    <time
                      dateTime={entry.created_at}
                      className="shrink-0 font-mono text-xs text-faint"
                    >
                      {formatMoment(entry.created_at)}
                    </time>
                  </li>
                ))}
              </ul>
              {more > 0 && (
                <p className="text-xs text-faint">{t('dashboard.sinceMore', { count: more })}</p>
              )}
            </>
          )
        }}
      </Loaded>
    </Widget>
  )
}
