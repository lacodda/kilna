import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { diffWords } from '@/lib/diff'
import { cn } from '@/lib/utils'

/**
 * What a text was against what it becomes, word by word, as one text
 * (v0.171): the words that go struck through on the bad ground, the words
 * that come on the good, the rest as they stand. For prose that is changed
 * by the word - a note - where two blocks one above the other leave the
 * reader to find the difference themselves.
 */
export function WordDiff({
  was,
  becomes,
  className,
}: {
  was: string
  becomes: string
  className?: string
}) {
  const { t } = useTranslation()
  const changes = useMemo(() => diffWords(was, becomes), [was, becomes])
  return (
    <p
      className={cn(
        'selectable max-h-48 overflow-y-auto rounded-md border border-line bg-bg px-2 py-1 leading-relaxed whitespace-pre-wrap',
        className,
      )}
    >
      {changes.map((change, index) =>
        change.kind === 'same' ? (
          // Pieces repeat - a word said twice - so the place is the identity.
          <span key={index}>{change.text}</span>
        ) : change.kind === 'removed' ? (
          <del
            key={index}
            title={t('assistant.noteWas')}
            className="rounded-xs bg-bad-soft text-dim line-through"
          >
            {change.text}
          </del>
        ) : (
          <ins
            key={index}
            title={t('assistant.noteBecomes')}
            className="rounded-xs bg-good-soft text-text no-underline"
          >
            {change.text}
          </ins>
        ),
      )}
    </p>
  )
}
