import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { MessageCircleQuestion } from 'lucide-react'
import { createChat, deleteChat, renameChat } from '@/lib/api/assistant'
import type { Chat, ChatSummary } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { PromptDialog } from '@/components/AppDialog'
import { ConfirmAction } from '@/components/ConfirmAction'

/*
 * The chats, as both places that list them read them: the drawer from the
 * title bar holds every chat of the profile, a card's Assistant tab holds its
 * work's. Until v0.77 each wrote the same queries, the same three writes and
 * the same two dialogs for itself; what differs now is only how the list is
 * drawn - rows in the drawer, chips on the tab.
 */

/**
 * The chats about `workId` - every chat of the profile when it is absent -
 * with whether the CLI is there, which of them are running, and the writes
 * that make, rename and delete one.
 */
export function useChats(
  workId: string | undefined,
  {
    onCreated,
    onRemoved,
  }: { onCreated?: (chat: Chat) => void; onRemoved?: (chatId: string) => void } = {},
) {
  const status = useQuery({
    ...queries.assistantStatus(),
    // Installing the CLI mid-session is rare; asking once a minute is plenty.
    staleTime: 60_000,
  })
  const summaries = useQuery({
    ...queries.chats(workId),
    // A chat an agent opened from outside the window (`kilna --mcp`) shows
    // up while the list is open, not on the next visit.
    refetchInterval: 30_000,
  })
  // Kept current by the run events' bridge on every run's start and end.
  const active = useQuery({ ...queries.activeRuns(), staleTime: 0 })

  const create = useAppMutation({
    mutationFn: () => createChat({ work_id: workId }),
    refresh: [keys.allChats],
    onSuccess: (chat) => onCreated?.(chat),
  })
  const rename = useAppMutation({
    mutationFn: ({ id, title }: { id: string; title: string | null }) => renameChat(id, title),
    refresh: [keys.allChats],
  })
  const remove = useAppMutation({
    mutationFn: (id: string) => deleteChat(id),
    refresh: [keys.allChats],
    onSuccess: (_, id) => onRemoved?.(id),
  })

  return {
    status: status.data,
    chats: summaries.data ?? [],
    pending: summaries.isPending,
    running: new Set(active.data ?? []),
    create,
    rename,
    remove,
  }
}

/** What a chat is doing, beside its name: a run in flight, a question waiting. */
export function ChatMarks({ chat, running }: { chat: ChatSummary; running: boolean }) {
  const { t } = useTranslation()
  return (
    <>
      {running && (
        <span aria-hidden className="size-1.5 shrink-0 animate-pulse rounded-full bg-accent" />
      )}
      {/* A question waiting in a chat you are not looking at is the thing
          this mark exists for; a run in flight already pulses. */}
      {chat.waiting_since !== undefined && (
        <MessageCircleQuestion
          aria-label={t('assistant.waitingMark')}
          className="size-3.5 shrink-0 text-accent-2"
        />
      )}
    </>
  )
}

/**
 * Renaming a chat and deleting one, as the menus of both lists ask for them.
 * `ask.rename(chat)` and `ask.remove(id)` open the questions.
 */
export function useChatQuestions(chats: ReturnType<typeof useChats>) {
  const { t } = useTranslation()
  const [renaming, setRenaming] = useState<ChatSummary | null>(null)
  const [removing, setRemoving] = useState<string | null>(null)

  const dialogs = (
    <>
      <PromptDialog
        open={renaming !== null}
        onOpenChange={(next) => {
          if (!next) setRenaming(null)
        }}
        title={t('assistant.renameTitle')}
        label={t('assistant.renameLabel')}
        initialValue={renaming?.title ?? ''}
        confirmLabel={t('dialog.save')}
        onSubmit={(value) => {
          if (renaming !== null) chats.rename.mutate({ id: renaming.id, title: value })
        }}
      />

      {/* A chat is deleted for good - it does not go to the trash - so this
          is a question that cannot be taken back, asked the way the app asks
          those: no dismissal by a stray click, the verb in the danger tone. */}
      <ConfirmAction
        open={removing !== null}
        onOpenChange={(next) => {
          if (!next) setRemoving(null)
        }}
        title={t('assistant.deleteTitle')}
        description={t('assistant.deleteBody')}
        actionLabel={t('assistant.delete')}
        pending={chats.remove.isPending}
        onConfirm={() => {
          if (removing !== null) chats.remove.mutate(removing)
          setRemoving(null)
        }}
      />
    </>
  )

  return { ask: { rename: setRenaming, remove: setRemoving }, dialogs }
}
