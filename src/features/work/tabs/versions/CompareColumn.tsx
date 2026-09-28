import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { X } from 'lucide-react'
import type { LineMark } from '@/components/ui/marked-text'
import { MarkedText } from '@/components/ui/marked-text'
import { Button } from '@/components/ui/button'
import { withSections } from '@/features/work/tabs/versions/metrics'
import { cn } from '@/lib/utils'

interface Props {
  /** Which version this is. */
  label: string
  body: string
  /** Its lines the text beside it no longer has. */
  removed: readonly LineMark[]
  counts: { added: number; removed: number } | null
  /** The type the text beside it is set in, so the two lines up. */
  metrics: string
  /** Put it away. Absent where it is not the reader's to put away - the
   *  source a new version is being written from. */
  onClose?: () => void
}

/**
 * The older side of a comparison: another version, standing to the right of
 * the one being read or written, with its lines that are gone marked.
 *
 * It scrolls with the text beside it rather than on its own - the two share
 * one scroller - so a line and the line it replaced stay level. Its head
 * sticks to the top of that scroller, so which version this is and how far
 * the two have moved apart stay in view down a long text.
 */
export function CompareColumn({ label, body, removed, counts, metrics, onClose }: Props) {
  const { t } = useTranslation()
  const lineMarks = useMemo(() => withSections(body, removed), [body, removed])

  return (
    <aside className="flex min-w-0 flex-1 flex-col border-l border-line">
      <div className="sticky top-0 z-10 flex items-center gap-2 border-b border-line bg-raise px-3 py-1 text-xs text-dim">
        <span className="truncate font-medium">{label}</span>
        {counts !== null && (
          <span className="ml-auto whitespace-nowrap text-faint">
            {counts.added === 0 && counts.removed === 0
              ? t('versions.diffSame')
              : [
                  t('versions.diffAdded', { count: counts.added }),
                  t('versions.diffRemoved', { count: counts.removed }),
                ].join(' · ')}
          </span>
        )}
        {onClose !== undefined && (
          <Button
            variant="icon"
            size="icon-sm"
            className={cn(counts === null && 'ml-auto')}
            title={t('versions.stopComparing')}
            aria-label={t('versions.stopComparing')}
            onClick={onClose}
          >
            <X aria-hidden />
          </Button>
        )}
      </div>
      <MarkedText text={body} lineMarks={lineMarks} className={cn(metrics, 'text-dim')} />
    </aside>
  )
}
