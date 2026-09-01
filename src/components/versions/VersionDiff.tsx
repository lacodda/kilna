import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { countChanges, diffLines } from '@/lib/diff'
import { cn } from '@/lib/utils'

interface Props {
  before: string
  after: string
  beforeLabel: string
  afterLabel: string
}

/**
 * Two versions, side by side, with what moved between them marked.
 *
 * One column each rather than a single merged column: the question being asked
 * is "how did this verse read before, and how does it read now", and an
 * interleaved diff answers a different one.
 */
export function VersionDiff({ before, after, beforeLabel, afterLabel }: Props) {
  const { t } = useTranslation()
  const changes = useMemo(() => diffLines(before, after), [before, after])
  const counts = useMemo(() => countChanges(changes), [changes])

  return (
    <div className="flex flex-col gap-2">
      <p className="text-xs text-dim">
        {counts.added === 0 && counts.removed === 0
          ? t('versions.diffSame')
          : // Two numbers, two phrases: one plural form cannot serve both, and
            // i18next pluralises on `count` rather than on a named value.
            [
              t('versions.diffAdded', { count: counts.added }),
              t('versions.diffRemoved', { count: counts.removed }),
            ].join(' · ')}
      </p>

      {/* Side by side only when each side has room to hold a line; stacked
          otherwise, which still answers "what changed" without wrapping every
          line in half. */}
      <div className="grid gap-3 [@media(min-width:56rem)]:grid-cols-2">
        <Column
          title={beforeLabel}
          changes={changes}
          // The left column is the old text: what stayed, and what left it.
          keep={(kind) => kind !== 'added'}
          markKind="removed"
        />
        <Column
          title={afterLabel}
          changes={changes}
          keep={(kind) => kind !== 'removed'}
          markKind="added"
        />
      </div>
    </div>
  )
}

function Column({
  title,
  changes,
  keep,
  markKind,
}: {
  title: string
  changes: ReturnType<typeof diffLines>
  keep: (kind: 'same' | 'added' | 'removed') => boolean
  markKind: 'added' | 'removed'
}) {
  return (
    <div className="overflow-hidden rounded-xl border border-line">
      <header className="border-b border-line px-3 py-1.5 text-xs font-medium text-dim">
        {title}
      </header>
      <div className="selectable max-h-[28rem] overflow-auto py-1 font-mono text-[13px] leading-relaxed">
        {changes.filter((change) => keep(change.kind)).map((change, index) => (
          <div
            // Lines repeat and reorder, so their text is not an identity; the
            // list is rebuilt whole on every change anyway.
            key={index}
            className={cn(
              'whitespace-pre-wrap px-3',
              change.kind === markKind && markKind === 'added' && 'bg-good-soft text-good',
              change.kind === markKind && markKind === 'removed' && 'bg-bad-soft text-bad',
            )}
          >
            {/* A blank line still needs height, or the columns drift apart. */}
            {change.text === '' ? ' ' : change.text}
          </div>
        ))}
      </div>
    </div>
  )
}
