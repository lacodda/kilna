import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { countChanges, diffLines } from '@/lib/diff'
import { cn } from '@/lib/utils'

interface Props {
  /** What the text would follow: its words and what to call it. Null when
   *  nothing stands in that role yet. */
  against: { label: string; body: string } | null
  /** The text about to be kept. */
  body: string
  /** What the role is called, for saying there is nothing before it. */
  role: string
}

/**
 * An answer against the version it would follow, before it is kept (v0.106):
 * the lines it brings in on the good ground, the lines it drops struck
 * through on the bad, the rest as they stand - and how many of each above.
 *
 * One column rather than the Versions tab's two side by side: a dialog has
 * room for one text, and a person deciding whether to keep an answer reads
 * what it changes, in order, not two pages to hold level.
 */
export function VersionDiff({ against, body, role }: Props) {
  const { t } = useTranslation()
  const changes = useMemo(
    () => (against === null ? null : diffLines(against.body, body)),
    [against, body],
  )

  if (against === null || changes === null) {
    return <p className="text-xs text-dim">{t('assistant.diffNothingBefore', { role })}</p>
  }
  const counts = countChanges(changes)

  return (
    <section aria-label={t('assistant.diffLabel')} className="flex min-h-0 flex-col gap-1.5">
      <p className="flex items-center gap-2 text-xs text-dim">
        <span className="truncate">{t('assistant.diffAgainst', { version: against.label })}</span>
        <span className="ml-auto whitespace-nowrap text-faint">
          {counts.added === 0 && counts.removed === 0
            ? t('versions.diffSame')
            : [
                t('versions.diffAdded', { count: counts.added }),
                t('versions.diffRemoved', { count: counts.removed }),
              ].join(' · ')}
        </span>
      </p>
      <ol className="selectable max-h-72 overflow-y-auto rounded-md border border-line bg-bg py-1 font-mono text-xs leading-relaxed">
        {changes.map((change, index) => (
          <li
            // Lines repeat - a chorus twice - so the place is the identity.
            key={index}
            data-change={change.kind}
            className={cn(
              'px-2 whitespace-pre-wrap',
              change.kind === 'added' && 'bg-good-soft',
              change.kind === 'removed' && 'bg-bad-soft text-dim line-through',
            )}
          >
            <span aria-hidden className="mr-2 inline-block w-2 text-faint select-none">
              {change.kind === 'added' ? '+' : change.kind === 'removed' ? '−' : ' '}
            </span>
            {change.text === '' ? ' ' : change.text}
          </li>
        ))}
      </ol>
    </section>
  )
}
