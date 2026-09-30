import { useTranslation } from 'react-i18next'
import { BookPlus } from 'lucide-react'
import { startTask } from '@/lib/api/assistant'
import { selectionAction } from '@/lib/actions'
import { keys } from '@/lib/query/keys'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { useAssistant } from '@/lib/useAssistant'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { READING_TEXT } from '@/features/work/tabs/versions/ReadingText'

/**
 * "To the canon": the lines selected in a version, handed to the profile's
 * action about a selection, whose answer proposes facts with this work and
 * version as their source and the lines as the quote (ADR 0043). Drawn only
 * where the craft has such an action.
 *
 * The selection is read when the button is pressed; pressing it does not take
 * the selection away, because the button never takes the focus from the text.
 */
export function ToCanonButton({ workId, versionId }: { workId: string; versionId: string }) {
  const { t } = useTranslation()
  const assistant = useAssistant()
  const action = selectionAction(useProfile().config.prompts ?? [])
  const start = useAppMutation({
    mutationFn: (selection: string) =>
      startTask(workId, action?.key ?? '', { versionId, selection }),
    refresh: [keys.activeTasks, keys.allChats],
    onSuccess: (started) => {
      say.info(t('assistant.taskStarted', { title: started.title }))
      assistant.open(started.chatId)
    },
  })
  if (action === undefined) return null

  const selected = (): string => {
    const selection = window.getSelection()
    if (selection === null || selection.rangeCount === 0) return ''
    const anchor = selection.anchorNode
    const inside =
      anchor !== null &&
      (anchor instanceof Element ? anchor : anchor.parentElement)?.closest(`[${READING_TEXT}]`) !==
        null
    return inside ? selection.toString().trim() : ''
  }

  const label = sayLabel(action.label)
  return (
    <Button
      variant="ghost"
      size="sm"
      title={t('canon.toCanonHint')}
      disabled={start.isPending}
      onMouseDown={(event) => event.preventDefault()}
      onClick={() => {
        const lines = selected()
        if (lines === '') say.info(t('canon.selectFirst'))
        else start.mutate(lines)
      }}
    >
      <BookPlus aria-hidden />
      <span className="@max-2xl:sr-only">{label}</span>
    </Button>
  )
}
