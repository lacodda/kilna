import type { TFunction } from 'i18next'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router'
import { journalForWork, type JournalEntry } from '@/lib/api'
import { keys } from '@/lib/query'
import { Badge } from '@/components/ui/badge'
import { EmptyState } from '@/components/ui/EmptyState'
import { SkeletonList } from '@/components/ui/Skeleton'
import { cn } from '@/lib/utils'
import type { Tab } from '@/components/card/tabs'

/** A `{{name}}` i18next left standing because the entry carried no such value,
 *  together with the quotes around it. */
const UNFILLED = /\s*[«"']?\{\{\s*[\w.]+\s*\}\}[»"']?\s*/g

/**
 * The sentence for an entry, built now rather than stored.
 *
 * An entry holds a key and its values, so a line written while the interface was
 * English reads in Russian the moment the language changes. A key with no
 * translation prints as itself instead of vanishing: a line of history nobody
 * worded is still a line of history.
 *
 * A value the entry does not carry is cut out rather than shown. i18next leaves
 * an unmatched `{{title}}` exactly as written, and the owner read
 * «{{title}}» удалено in his own history: the line still says that a work was
 * deleted, and the braces only say that whoever wrote the line lost the name.
 * Rows already written cannot get the value back, so the reading side is the
 * only place this can be fixed at all.
 *
 * The quotes go with the hole: «» left standing empty reads as a work whose
 * name is blank, rather than one whose name was never kept.
 */
export function sentence(entry: JournalEntry, t: TFunction): string {
  const key = `journal.${entry.action}`
  const said = t(key, entry.params)
  if (said === key) return entry.action

  // Replace and compare, rather than `test` and then replace: a global regex
  // carries `lastIndex` between calls, so testing first would skip every
  // other line it was asked about.
  const filled = said.replace(UNFILLED, ' ')
  return filled === said ? said : filled.replace(/\s+/g, ' ').trim()
}

/** Time of day for today's entries, date for older ones. */
export function when(timestamp: string, locale: string): string {
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

/**
 * Which tab of a work answers for a line of the journal.
 *
 * The action names it: a score was added on the scoring tab, a scene on the
 * board, a release in the calendar's own tab. Reading the prefix rather than
 * listing every action keeps a new `score.something` pointing at the right
 * place without anyone remembering to come back here; an action whose prefix
 * is not one of these opens the work where it opens by default.
 */
const TAB_FOR_ACTION: Record<string, Tab> = {
  score: 'score',
  version: 'versions',
  scene: 'scenes',
  cut: 'scenes',
  release: 'releases',
  link: 'links',
  asset: 'files',
  note: 'notes',
  proposal: 'assistant',
  assistant: 'assistant',
  tier: 'score',
}

/** Where a line points, or null when it is not about one work. */
function destinationOf(entry: JournalEntry): string | null {
  if (entry.entity !== 'work' || entry.entity_id === null) return null
  const tab = TAB_FOR_ACTION[entry.action.split('.')[0] ?? '']
  return tab === undefined ? `/works/${entry.entity_id}` : `/works/${entry.entity_id}/${tab}`
}

function Line({ entry }: { entry: JournalEntry }) {
  const { t, i18n } = useTranslation()
  const needsALook = entry.level === 'warn' && entry.read_at === null
  const to = destinationOf(entry)

  const body = (
    <>
      {sentence(entry, t)}
      {entry.occurrences > 1 && (
        <Badge variant="soft" className="ml-2">
          {t('journal.repeated', { count: entry.occurrences })}
        </Badge>
      )}
    </>
  )

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
      {/* A line about a work opens that work, on the tab the line is about:
          reading "scored 78" and then hunting the catalogue for the song it
          was about is the walk the owner asked to be rid of. A line about
          nothing in particular stays plain text rather than becoming a link
          that goes nowhere. */}
      {to === null ? (
        <p className={cn('min-w-0 flex-1 text-sm', entry.level === 'warn' && 'text-text')}>
          {body}
        </p>
      ) : (
        <Link
          to={to}
          className={cn(
            'min-w-0 flex-1 text-sm text-text no-underline hover:underline',
            entry.level === 'warn' && 'text-text',
          )}
        >
          {body}
        </Link>
      )}
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
