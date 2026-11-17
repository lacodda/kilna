import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { ProposedWord, WordsPackage } from '@/lib/api/types'
import { sayReason } from '@/lib/errors'
import { queries } from '@/lib/query/queries'
import { Checkbox } from '@/components/ui/checkbox'

interface Props {
  pack: WordsPackage
  /** The items kept - `word:0`, `word:2` - by the key an apply names them with. */
  chosen: readonly string[]
  onChosen?: (items: string[]) => void
  /** Answered already: the words are read, not chosen. */
  answered?: boolean
}

/**
 * Words proposed for the record, one item each (ADR 0052): the word, then
 * what keeping it would write - a phrase and the shelf it goes on (ADR 0064),
 * how it is sung, how strictly the register takes it, the works a meaning is
 * in - and why it was proposed. A box beside each keeps it or leaves it out, the way a
 * package for the canon is read: one wrong word must not cost the right ones.
 *
 * The same list under an answer in a chat and in the dialog that reads the
 * owner's own texts, so a word proposed reads alike wherever it came from.
 */
export function WordsPackageView({ pack, chosen, onChosen, answered = false }: Props) {
  const { t } = useTranslation()

  const toggle = (item: string, on: boolean) =>
    onChosen?.(on ? [...chosen, item] : chosen.filter((one) => one !== item))

  return (
    <div className="flex flex-col gap-2 text-xs">
      <ul className="flex flex-col gap-1.5">
        {pack.words.map((word, index) => {
          const item = `word:${String(index)}`
          return (
            // Beside the word rather than wrapped round it with a label: the
            // box is named by its word alone, not by everything said under it.
            <li key={item} className="flex items-start gap-2">
              {!answered && onChosen !== undefined && (
                <Checkbox
                  checked={chosen.includes(item)}
                  onCheckedChange={(on) => toggle(item, on)}
                  aria-label={t('words.keepItem', { word: word.word })}
                  className="mt-0.5"
                />
              )}
              <Word word={word} />
            </li>
          )
        })}
      </ul>

      {/* A word that reads as several, written with a comma or a slash:
          kept as proposed if the person says so, but said first. */}
      {(pack.warnings ?? []).length > 0 && !answered && (
        <section className="flex flex-col gap-0.5 text-warn">
          {(pack.warnings ?? []).map((reason, index) => (
            <span key={String(index)}>{sayReason(reason)}</span>
          ))}
        </section>
      )}

      {/* What the proposer named and the workspace could not take - a word
          with no letters, a work nobody can find - said rather than dropped.
          Once answered it has had its say. */}
      {(pack.dropped ?? []).length > 0 && !answered && (
        <section className="flex flex-col gap-0.5 text-warn">
          <b className="text-2xs tracking-caption uppercase">{t('words.leftOut')}</b>
          {(pack.dropped ?? []).map((reason, index) => (
            <span key={String(index)}>{sayReason(reason)}</span>
          ))}
        </section>
      )}
    </div>
  )
}

/** One word: what it is, what keeping it writes, and why it was proposed. */
function Word({ word }: { word: ProposedWord }) {
  const { t } = useTranslation()
  // A shelf nobody has made yet is made on keeping (ADR 0064), and the
  // person should know a name is about to become a shelf.
  const shelves = useQuery(queries.shelves())
  const known = (name: string) =>
    (shelves.data ?? []).some(
      (shelf) => shelf.name.toLocaleLowerCase() === name.trim().toLocaleLowerCase(),
    )

  // A word the record already keeps: the item changes it, and what it
  // changes is read against what the record says - was, becomes (v0.116).
  const action = word.action ?? 'upsert'
  if (action !== 'upsert' || word.known === true) {
    const changes = word.changes ?? []
    return (
      <span className="flex min-w-0 flex-1 flex-col gap-0.5">
        <span className="flex flex-wrap items-baseline gap-x-2">
          <b className="text-sm font-semibold text-text">{word.word}</b>
          <span className="text-warn">
            {action === 'delete'
              ? t('words.takeOut')
              : action === 'split'
                ? t('words.splitInto', { parts: (word.into ?? []).join(', ') })
                : t('words.changes')}
          </span>
        </span>
        {action === 'upsert' &&
          changes.map((change) => (
            <span key={change.field} className="flex flex-wrap items-baseline gap-x-1.5">
              <span className="text-faint">
                {t(FIELD_KEYS[change.field] ?? 'words.field.other')}
              </span>
              <span className="text-faint line-through">{change.was}</span>
              <span aria-hidden className="text-faint">
                →
              </span>
              <span className="text-text">{change.becomes}</span>
            </span>
          ))}
        {action === 'upsert' && changes.length === 0 && (
          <span className="text-faint">{t('words.nothingChanges')}</span>
        )}
        {word.note !== undefined && word.note !== '' && action !== 'upsert' && (
          <span className="text-faint">{word.note}</span>
        )}
      </span>
    )
  }

  const writes = [
    word.phrase === true && word.shelf !== undefined
      ? t(known(word.shelf) ? 'words.toShelf' : 'words.toNewShelf', { shelf: word.shelf })
      : word.phrase === true
        ? t('words.toPhrase')
        : null,
    word.strictness !== undefined
      ? t('words.toRegister', { strictness: t(`register.strictnesses.${word.strictness}`) })
      : null,
    (word.forms ?? []).length > 0
      ? t('words.forms', { forms: (word.forms ?? []).join(', ') })
      : null,
    (word.works ?? []).length > 0
      ? t('words.works', {
          works: (word.works ?? [])
            .map((work) => t('words.workTitle', { title: work.title }))
            .join(', '),
        })
      : null,
  ].filter((part) => part !== null)

  // A word sung one way, in the form it is written - the commonest case, and
  // every word the texts propose - reads on one line: the word, then how it
  // is sung. Other forms each get a line of their own.
  const sung = word.sung ?? []
  const own = sung.length === 1 && sung[0]?.written === word.word ? sung[0] : null

  return (
    <span className="flex min-w-0 flex-1 flex-col gap-0.5">
      <span className="flex flex-wrap items-baseline gap-x-2">
        <b className="text-sm font-semibold text-text">{word.word}</b>
        {own !== null && <Sung sung={own.sung} />}
        {word.kind !== undefined && (
          <span className="text-faint">{t(`register.kinds.${word.kind}`)}</span>
        )}
      </span>
      {own === null &&
        sung.map((pair) => (
          <span key={`${pair.written}→${pair.sung}`} className="flex items-baseline gap-1.5">
            <span className="text-dim">{pair.written}</span>
            <Sung sung={pair.sung} />
          </span>
        ))}
      {writes.length > 0 && <span className="text-dim">{writes.join(' · ')}</span>}
      {word.note !== undefined && word.note !== '' && (
        <span className="text-faint">{word.note}</span>
      )}
      {/* Where kilna found it written this way in your own texts: the
          titles come as data, the sentence is the window's. */}
      {(word.seen_in ?? []).length > 0 && (
        <span className="text-faint">
          {t('words.seenIn', {
            titles: (word.seen_in ?? [])
              .slice(0, 3)
              .map((title) => t('words.workTitle', { title }))
              .join(', '),
            count: (word.seen_in ?? []).length,
          })}
        </span>
      )}
    </span>
  )
}

/** A word per field a change names; one the window does not know reads as
 * "something else". */
const FIELD_KEYS: Record<string, string> = {
  strictness: 'words.field.strictness',
  forms: 'words.field.forms',
  kind: 'words.field.kind',
  topic: 'words.field.topic',
  note: 'words.field.note',
  sung: 'words.field.sung',
  works: 'words.field.works',
}

/** How a form is sung, after its arrow - in the mono the editor of sung forms
 *  uses, where a capital vowel reads as the stress it is. */
function Sung({ sung }: { sung: string }) {
  return (
    <span className="flex items-baseline gap-1.5">
      <span aria-hidden className="text-faint">
        →
      </span>
      <span className="font-mono text-sm text-text">{sung}</span>
    </span>
  )
}
