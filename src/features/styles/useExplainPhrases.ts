import { useTranslation } from 'react-i18next'
import { startPhrasesTask } from '@/lib/api/styles'
import type { PhraseAsked, PromptTemplate } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { useAssistant } from '@/lib/useAssistant'
import { say as sayLabel, useProfile } from '@/lib/useProfile'

/**
 * "Explain": phrases the dictionary does not know, handed to the profile's
 * action about phrases (v0.94), which answers with bricks to keep. The action
 * is found by what it is about, not by its key - a profile names its own.
 * The drawer opens on the chat, so the answer is watched arriving.
 */
export function useExplainPhrases(composition: string | undefined) {
  const { t } = useTranslation()
  const assistant = useAssistant()
  const action: PromptTemplate | undefined = useProfile().config.prompts.find(
    (prompt) => prompt.scope === 'phrases',
  )
  const start = useAppMutation({
    mutationFn: (phrases: PhraseAsked[]) => {
      if (composition === undefined || action === undefined) {
        return Promise.reject(new Error('no action explains phrases'))
      }
      return startPhrasesTask(composition, phrases, action.key)
    },
    refresh: [keys.activeTasks, keys.allChats],
    onSuccess: (started) => {
      say.info(t('assistant.taskStarted', { title: started.title }))
      assistant.open(started.chatId)
    },
  })
  return {
    /** Absent when the profile has no action about phrases. */
    label: action === undefined ? undefined : sayLabel(action.label),
    explain: (phrases: PhraseAsked[]) => start.mutate(phrases),
    pending: start.isPending,
  }
}
