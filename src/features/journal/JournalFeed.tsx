import type { TFunction } from 'i18next'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import type { JournalEntry } from '@/lib/api/types'
import { Chip, type ChipProps } from '@/components/ui/chip'
import { ListRow } from '@/components/ui/list-row'
import { cn } from '@/lib/utils'
import { formatStamp } from '@/lib/format'
import { journalLook, needsALook, type JournalTone } from '@/lib/journalLook'
import type { Tab } from '@/features/work/tabs'

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
  // Its own tab since v0.73; the line used to open the storyboard.
  cut: 'cuts',
  release: 'overview',
  link: 'links',
  asset: 'files',
  note: 'notes',
  proposal: 'assistant',
  assistant: 'assistant',
  tier: 'score',
}

/** Where a line points, or null when it is not about one work. */
export function destinationOf(entry: JournalEntry): string | null {
  if (entry.entity !== 'work' || entry.entity_id === null) return null
  const tab = TAB_FOR_ACTION[entry.action.split('.')[0] ?? '']
  return tab === undefined ? `/works/${entry.entity_id}` : `/works/${entry.entity_id}/${tab}`
}

/** The tile a line's glyph sits on, one tint per tone: the mockup's `.jic`. */
const TILE: Record<JournalTone, string> = {
  good: 'bg-good-soft text-good',
  accent: 'bg-accent-soft text-accent',
  warn: 'bg-warn-soft text-warn',
  dim: 'bg-soft text-dim',
}

/**
 * One line of history, in the mockup's anatomy: a tinted tile saying what
 * kind of thing happened, the sentence with a quieter line under it, and the
 * moment at the end.
 */
function Line({ entry }: { entry: JournalEntry }) {
  const { t } = useTranslation()
  const look = journalLook(entry)
  const to = destinationOf(entry)
  const said = sentence(entry, t)

  // The quieter line: the area the line is about, and how many times it
  // happened. The count used to be a badge beside the sentence, which a long
  // sentence pushed out of sight; under it, it is always where the eye
  // finishes the line.
  const under = [
    look.kind === null ? null : t(look.kind),
    entry.occurrences > 1 ? t('journal.repeated', { count: entry.occurrences }) : null,
  ]
    .filter((part) => part !== null)
    .join(' · ')

  return (
    <ListRow
      render={<li />}
      // The dot is placed against the row, in its left padding.
      className="relative items-start"
      start={
        <>
          {/* A dot rather than a word: the feed is scanned, not read. It
              marks the warnings nobody has marked seen - the ones the
              filter above counts. */}
          {needsALook(entry) && (
            <span className="absolute top-4 left-1 size-1.5 rounded-full bg-accent">
              <span className="sr-only">{t('journal.unreadOnly')}</span>
            </span>
          )}
          <span
            aria-hidden
            className={cn('grid size-6.5 place-items-center rounded-md', TILE[look.tone])}
          >
            <look.glyph className="size-3.5" />
          </span>
        </>
      }
      description={under === '' ? undefined : under}
      end={
        <time dateTime={entry.created_at} title={entry.created_at} className="font-mono text-2xs">
          {formatStamp(entry.created_at)}
        </time>
      }
    >
      {/* A line about a work opens that work, on the tab the line is about:
          reading "scored 78" and then hunting the catalogue for the song it
          was about is the walk the owner asked to be rid of. A line about
          nothing in particular stays plain text rather than becoming a link
          that goes nowhere. */}
      {to === null ? (
        <span className="font-semibold" title={said}>
          {said}
        </span>
      ) : (
        <Link to={to} title={said} className="font-semibold text-text no-underline hover:underline">
          {said}
        </Link>
      )}
    </ListRow>
  )
}

/** The lines themselves, given entries someone else fetched. What an empty
 *  feed says is the caller's: it differs per surface. */
export function JournalLines({ entries }: { entries: JournalEntry[] }) {
  return (
    <ul className="flex flex-col">
      {entries.map((entry) => (
        <Line key={entry.id} entry={entry} />
      ))}
    </ul>
  )
}

/** The chip a line's area is named in, one tint per tone - the tile's tints,
 *  in the mockup's `.chip.acc`/`.good`/`.warn` and a plain one. */
const CHIP: Record<JournalTone, NonNullable<ChipProps['variant']>> = {
  good: 'good',
  accent: 'accent',
  warn: 'warn',
  dim: 'outline',
}

/**
 * The chip naming the area a line of the journal is about, in its tone - how
 * a line stands among the moments of a work's axis, where every line is
 * about the same work and the area is the one thing left to tell them apart
 * by. An action no area claims has none, rather than one that names nothing.
 */
export function JournalChip({ entry }: { entry: JournalEntry }) {
  const { t } = useTranslation()
  const look = journalLook(entry)
  if (look.kind === null) return null
  return <Chip variant={CHIP[look.tone]}>{t(look.kind)}</Chip>
}
