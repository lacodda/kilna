import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { keepWords } from '@/lib/api/register'
import type { WordsPackage } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { wordItems } from '@/lib/words'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { SkeletonList } from '@/components/ui/skeleton'
import { Dialog } from '@/components/AppDialog'
import { Loaded } from '@/components/Loaded'
import { WordsPackageView } from '@/features/words/WordsPackageView'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
}

/**
 * What the owner's sung texts already say about singing, proposed for the
 * record (ADR 0052, ADR 0053): a capital vowel the dictionary does not put
 * there, a respelling - "МарсЭль" for "Марсель" - each with the works it was
 * written in. Every word is ticked; the owner unticks what was a one-off and
 * keeps the rest in one write.
 *
 * Read afresh on every opening: the texts are what it reads, and a verse
 * rewritten a minute ago is what the person opened it to see. Mounted only
 * while open, so the boxes start ticked each time.
 */
export function FromTextsDialog({ open, onOpenChange }: Props) {
  const { t } = useTranslation()
  const found = useQuery(queries.wordsFromTexts())
  // What was unticked, rather than what is ticked: the words arrive after the
  // dialog opens, and every one of them starts kept.
  const [left, setLeft] = useState<readonly string[]>([])

  const all = found.data === undefined ? [] : wordItems(found.data)
  const chosen = all.filter((item) => !left.includes(item))

  const keep = useAppMutation({
    mutationFn: ({ pkg, items }: { pkg: WordsPackage; items: string[] | null }) =>
      keepWords(pkg, items),
    failure: 'words.keepFailed',
    refresh: refresh.term,
    onSuccess: (ids) => {
      say.ok(t('words.kept', { count: ids.length }))
      onOpenChange(false)
    },
  })

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={t('words.fromTextsTitle')}
      description={t('words.fromTextsBody')}
      size="lg"
      dirty={left.length > 0}
      footer={
        all.length > 0 && found.data !== undefined ? (
          <Button
            variant="primary"
            disabled={chosen.length === 0 || keep.isPending}
            onClick={() => {
              if (found.data === undefined) return
              // Whole when nothing was unticked: the backend keeps the package
              // as it read it, rather than a list that happens to name all.
              keep.mutate({ pkg: found.data, items: chosen.length === all.length ? null : chosen })
            }}
          >
            {t('words.keep', { n: chosen.length })}
          </Button>
        ) : undefined
      }
    >
      <Loaded
        query={found}
        skeleton={<SkeletonList rows={4} />}
        isEmpty={(pkg) => pkg.words.length === 0}
        emptyState={<EmptyState plain title={t('words.fromTextsNone')} className="py-2" />}
        plain
      >
        {(pkg) => (
          <WordsPackageView
            pack={pkg}
            chosen={chosen}
            onChosen={(items) => setLeft(all.filter((item) => !items.includes(item)))}
          />
        )}
      </Loaded>
    </Dialog>
  )
}
