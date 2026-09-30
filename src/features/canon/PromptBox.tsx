import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Pencil, Sparkles } from 'lucide-react'
import { describeCard } from '@/lib/api/canon'
import type { CardView } from '@/lib/api/types'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { Button } from '@/components/ui/button'
import { CopyButton } from '@/components/ui/copy-button'
import { Textarea } from '@/components/ui/textarea'
import { cn } from '@/lib/utils'

interface Props {
  view: CardView
  /** Start the assistant's description, when the profile has the action. */
  onDescribe?: () => void
  describing: boolean
}

/**
 * The description a picture generator is given for the card, in English,
 * instead of its name - written from the facts of the sections its kind is
 * described from, and marked stale when those facts move on.
 *
 * Written by the assistant or by hand; either way it answers to the facts as
 * they stood when it was written, and the mark says when they no longer do.
 */
export function PromptBox({ view, onDescribe, describing }: Props) {
  const { t } = useTranslation()
  const [editing, setEditing] = useState(false)
  const [text, setText] = useState(view.card.prompt ?? '')
  const prompt = view.card.prompt

  const save = useAppMutation({
    mutationFn: () => describeCard(view.card.id, text.trim() === '' ? null : text.trim()),
    failure: 'toast.cardSaveFailed',
    refresh: refresh.canon,
    onSuccess: () => setEditing(false),
  })

  return (
    <div className="mt-1.5 ml-8.5 flex flex-col gap-1.5 rounded-lg border border-line bg-bg px-2.5 py-2">
      <header className="flex flex-wrap items-center gap-2">
        <span className="text-2xs font-semibold tracking-caption text-faint uppercase">
          {t('canon.forGenerator')}
        </span>
        {prompt !== null && (
          <span
            className={cn(
              'rounded-sm px-1 text-2xs',
              view.prompt_stale ? 'bg-warn-soft text-warn' : 'bg-good-soft text-good',
            )}
          >
            {view.prompt_stale ? t('canon.promptStale') : t('canon.promptFresh')}
          </span>
        )}
        <span className="ml-auto flex items-center gap-1">
          {prompt !== null && (
            <CopyButton
              value={prompt}
              label={t('canon.copyPrompt')}
              copiedLabel={t('canon.promptCopied')}
            />
          )}
          <Button
            size="icon-xs"
            variant="icon"
            aria-label={t('canon.editPrompt')}
            title={t('canon.editPrompt')}
            onClick={() => {
              setText(prompt ?? '')
              setEditing(!editing)
            }}
          >
            <Pencil aria-hidden />
          </Button>
          {onDescribe !== undefined && (
            <Button size="xs" variant="soft" onClick={onDescribe} disabled={describing}>
              <Sparkles aria-hidden />
              {prompt === null ? t('canon.describe') : t('canon.describeAgain')}
            </Button>
          )}
        </span>
      </header>
      {editing ? (
        <>
          <Textarea
            autoFocus
            autoResize
            rows={3}
            value={text}
            onChange={(event) => setText(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Escape') setEditing(false)
              if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') save.mutate()
            }}
            aria-label={t('canon.forGenerator')}
            className="font-mono text-xs"
          />
          <div className="flex gap-2">
            <Button
              size="xs"
              variant="primary"
              disabled={save.isPending}
              onClick={() => save.mutate()}
            >
              {t('dialog.save')}
            </Button>
            <Button size="xs" variant="ghost" onClick={() => setEditing(false)}>
              {t('dialog.cancel')}
            </Button>
          </div>
        </>
      ) : prompt === null ? (
        <p className="text-xs text-faint">{t('canon.noPrompt')}</p>
      ) : (
        <p className="selectable font-mono text-xs leading-relaxed text-dim">{prompt}</p>
      )}
    </div>
  )
}
