import type { TFunction } from 'i18next'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { journalForWork, type JournalEntry } from '@/lib/api'
import { keys } from '@/lib/query'
import { Badge } from '@/components/ui/Badge'
import { EmptyState } from '@/components/ui/EmptyState'
import { SkeletonList } from '@/components/ui/Skeleton'
import { cn } from '@/lib/utils'

/**
 * The sentence for an entry, built now rather than stored.
 *
 * An entry holds a key and its values, so a line written while the interface was
 * English reads in Russian the moment the language changes. A key with no
 * translation prints as itself instead of vanishing: a line of history nobody
 * worded is still a line of history.
 */
export function sentence(entry: JournalEntry, t: TFunction): string {
  const key = `journal.${entry.action}`
  const said = t(key, entry.params)
  return said === key ? entry.action : said
}

/** Time of day for today's entries, date for older ones. */
function when(timestamp: string, locale: string): string {
  const at = new Date(timestamp)
  if (Number.isNaN(at.getTime())) return timestamp

  const today = new Date()
  const sameDay =
    at.getFullYear() === today.getFullYear() &&
    at.getMonth() === today.getMonth() &&
    at.getDate() === today.getDate()

  return sameDay
    ? at.toLocaleTimeString(locale, { hour: '2-digit', minute: '2-digit' })
    : at.toLocaleDateString(locale, { day: 'numeric', month: 'short' })
}

function Line({ entry }: { entry: JournalEntry }) {
  const { t, i18n } = useTranslation()
  const needsALook = entry.level === 'warn' && entry.read_at === null

  return (
    <li className="flex items-baseline gap-3 border-b border-line py-2 last:border-b-0">
      {/* A dot rather than a word: the feed is scanned, not read. */}
      <span
        aria-hidden
        className={cn(
          'mt-1.5 size-1.5 shrink-0 rounded-full',
          needsALook ? 'bg-warn' : 'bg-line-2',
        )}
      />
      <p className={cn('min-w-0 flex-1 text-sm', entry.level === 'warn' && 'text-text')}>
        {sentence(entry, t)}
        {entry.occurrences > 1 && (
          <Badge variant="soft" className="ml-2">
            {t('journal.repeated', { count: entry.occurrences })}
          </Badge>
        )}
      </p>
      <time
        dateTime={entry.created_at}
        title={entry.created_at}
        className="shrink-0 text-xs tabular-nums text-faint"
      >
        {when(entry.created_at, i18n.language)}
      </time>
    </li>
  )
}

interface Props {
  entries: JournalEntry[]
  /** Shown when there is nothing yet — the wording differs per surface. */
  emptyTitle: string
  emptyBody?: string
}

/** The lines themselves, given entries someone else fetched. */
export function JournalLines({ entries, emptyTitle, emptyBody }: Props) {
  if (entries.length === 0) return <EmptyState title={emptyTitle} body={emptyBody} />

  return (
    <ul className="flex flex-col">
      {entries.map((entry) => (
        <Line key={entry.id} entry={entry} />
      ))}
    </ul>
  )
}

/**
 * One work's own history, on its card.
 *
 * A panel among the others for now; it becomes the card's History tab when the
 * card is split into tabs in v0.16. Fetches on its own rather than riding along
 * with the card's other queries — history is the part of a card nobody reads
 * every time.
 */
export function WorkHistory({ workId }: { workId: string }) {
  const { t } = useTranslation()
  const entries = useQuery({
    queryKey: keys.journalForWork(workId),
    queryFn: () => journalForWork(workId),
  })

  return (
    <section className="flex flex-col gap-3">
      <h3 className="text-sm font-semibold">{t('journal.title')}</h3>

      {entries.isPending && <SkeletonList rows={3} />}

      {entries.isError && (
        <p role="alert" className="text-sm text-bad">
          {t('toast.loadFailed')}
        </p>
      )}

      {entries.data != null && (
        <JournalLines
          entries={entries.data}
          emptyTitle={t('empty.historyTitle')}
          emptyBody={t('empty.historyBody')}
        />
      )}
    </section>
  )
}
