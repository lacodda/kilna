import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { BookPlus, Eye } from 'lucide-react'
import type { Composition } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { say as sayLabel } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { EmptyState } from '@/components/ui/empty-state'
import { SkeletonList } from '@/components/ui/skeleton'
import { Dialog } from '@/components/AppDialog'
import { Loaded } from '@/components/Loaded'
import { AddPhraseDialog } from '@/features/styles/AddPhraseDialog'
import { TaskPreviewDialog } from '@/features/assistant/TaskPreviewDialog'
import { useAssistant } from '@/lib/useAssistant'
import { useProfile } from '@/lib/useProfile'
import { useExplainPhrases } from '@/features/styles/useExplainPhrases'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  composition: Composition
}

/** How many phrases are ticked when the list opens: the most widely written. */
const TICKED = 20

/**
 * The owner's own dictionary, read out of their own texts (v0.94): every tag
 * the texts of the composition's role say that the dictionary does not know,
 * most widely written first, each with how many works and versions say it.
 * The ticked ones go to "Explain" in one task, whose answer is bricks kept
 * one click each; one can be kept by hand straight from here.
 *
 * Read afresh on every opening: the texts are what it reads.
 */
export function FromStylesDialog({ open, onOpenChange, composition }: Props) {
  const { t } = useTranslation()
  const found = useQuery(queries.phrasesFromTexts(composition.key))
  const explain = useExplainPhrases(composition.key)
  // What was ticked by hand, over the first ones ticked by default.
  const [toggled, setToggled] = useState<Record<string, boolean>>({})
  const [adding, setAdding] = useState<string | null>(null)
  const [previewing, setPreviewing] = useState(false)
  const assistant = useAssistant()
  const action = useProfile().config.prompts.find((prompt) => prompt.scope === 'phrases')

  const all = found.data ?? []
  const ticked = (phrase: string, index: number) => toggled[phrase] ?? index < TICKED
  const chosen = all.filter((one, index) => ticked(one.phrase, index))

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={t('phrases.fromTextsTitle', { name: sayLabel(composition.label) })}
      description={t('phrases.fromTextsBody')}
      size="lg"
      footer={
        explain.label === undefined || all.length === 0 ? undefined : (
          <>
            <Button
              variant="icon"
              size="icon-sm"
              title={t('assistant.previewTask', { label: explain.label })}
              aria-label={t('assistant.previewTask', { label: explain.label })}
              disabled={chosen.length === 0}
              onClick={() => setPreviewing(true)}
            >
              <Eye aria-hidden />
            </Button>
            <Button
              variant="primary"
              disabled={chosen.length === 0 || explain.pending}
              onClick={() => {
                explain.explain(chosen.map((one) => ({ phrase: one.phrase, count: one.versions })))
                onOpenChange(false)
              }}
            >
              {t('phrases.explainN', { label: explain.label, count: chosen.length })}
            </Button>
          </>
        )
      }
    >
      <Loaded
        query={found}
        skeleton={<SkeletonList rows={6} />}
        isEmpty={(list) => list.length === 0}
        emptyState={<EmptyState plain title={t('phrases.fromTextsNone')} className="py-2" />}
        plain
      >
        {(list) => (
          <ul className="flex max-h-[55vh] flex-col gap-1 overflow-y-auto pr-1 text-xs">
            {list.map((one, index) => (
              <li key={one.phrase} className="flex items-center gap-2">
                <Checkbox
                  checked={ticked(one.phrase, index)}
                  onCheckedChange={(on) => setToggled((now) => ({ ...now, [one.phrase]: on }))}
                  aria-label={t('phrases.pick', { phrase: one.phrase })}
                />
                <span className="flex min-w-0 flex-1 flex-col">
                  <span className="truncate font-mono text-sm text-text">{one.phrase}</span>
                  <span className="truncate text-faint">
                    {t('phrases.writtenIn', { works: one.works, versions: one.versions })}
                    {one.seen_in.length > 0 && ` · ${one.seen_in.join(', ')}`}
                  </span>
                </span>
                <Button
                  variant="icon"
                  size="icon-sm"
                  title={t('phrases.addByHand')}
                  aria-label={t('phrases.addByHandOf', { phrase: one.phrase })}
                  onClick={() => setAdding(one.phrase)}
                >
                  <BookPlus aria-hidden />
                </Button>
              </li>
            ))}
          </ul>
        )}
      </Loaded>
      {previewing && action !== undefined && (
        <TaskPreviewDialog
          open
          onOpenChange={setPreviewing}
          target={{
            on: 'phrases',
            composition: composition.key,
            phrases: chosen.map((one) => ({ phrase: one.phrase, count: one.versions })),
          }}
          action={action}
          onStarted={(started) => {
            setPreviewing(false)
            onOpenChange(false)
            assistant.open(started.chatId)
          }}
        />
      )}
      {adding !== null && (
        <AddPhraseDialog
          open
          onOpenChange={(next) => {
            if (!next) setAdding(null)
          }}
          composition={composition}
          phrase={adding}
        />
      )}
    </Dialog>
  )
}
