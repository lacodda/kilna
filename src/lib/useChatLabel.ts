import { useContext } from 'react'
import { useTranslation } from 'react-i18next'
import type { ChatSummary } from '@/lib/api/types'
import { chatLabel } from '@/lib/chat'
import { ProfileContext } from '@/lib/useProfile'

/**
 * What a chat is called, in the window's language.
 *
 * A hook because the name of a task's chat carries a word of the profile's -
 * the action that opened it - and everything that lists chats has to say it
 * the same way. Read from the context rather than through `useProfile`: the
 * banner of chats that wait stands above the workspace, where there may be
 * no profile yet, and a name as it was written is better than none.
 */
export function useChatLabel(): (summary: ChatSummary) => string {
  const { t } = useTranslation()
  const config = useContext(ProfileContext)?.config
  return (summary) => chatLabel(summary, t('assistant.untitled'), config)
}
