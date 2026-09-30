import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { Lens } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { Dialog } from '@/components/AppDialog'
import { CopyButton } from '@/components/ui/copy-button'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { Skeleton } from '@/components/ui/skeleton'

/**
 * A card as a task's prompt receives it: the very text the backend composes,
 * through the same lens, so what is read here is what the assistant is
 * handed - an empty page for a card the task may not know exists.
 */
export function SeenDialog({
  open,
  onOpenChange,
  cardId,
  lens: first,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
  cardId: string
  lens: Lens
}) {
  const { t } = useTranslation()
  const [lens, setLens] = useState<Lens>(first)
  const seen = useQuery({ ...queries.cardAsSeen(cardId, lens), enabled: open, staleTime: 0 })

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={t('canon.seeAsTask')}
      description={t('canon.seeAsTaskBody')}
      size="lg"
    >
      <div className="flex items-center gap-2">
        <SegmentedControl
          aria-label={t('canon.lens')}
          value={lens}
          onValueChange={(next) => setLens(next as Lens)}
        >
          {(['cover', 'work', 'public'] as const).map((one) => (
            <Segment key={one} value={one}>
              {t(`canon.lensName.${one}`)}
            </Segment>
          ))}
        </SegmentedControl>
        {seen.data !== undefined && seen.data !== '' && (
          <CopyButton
            className="ml-auto"
            value={seen.data}
            label={t('canon.copySeen')}
            copiedLabel={t('canon.promptCopied')}
          />
        )}
      </div>
      {seen.data === undefined ? (
        <Skeleton className="h-40 w-full" />
      ) : seen.data === '' ? (
        <p className="text-sm text-faint">{t('canon.seenNothing')}</p>
      ) : (
        <pre className="selectable max-h-[50vh] overflow-auto rounded-lg border border-line bg-bg p-3 font-mono text-xs leading-relaxed whitespace-pre-wrap text-dim">
          {seen.data}
        </pre>
      )}
    </Dialog>
  )
}
