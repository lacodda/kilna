import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { listen } from '@tauri-apps/api/event'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { MessageCircleQuestion, Plus } from 'lucide-react'
import {
  activeRuns,
  assistantStatus,
  createChat,
  deleteChat,
  listChatSummaries,
  renameChat,
  type RunEmission,
} from '@/lib/api'
import { chatLabel } from '@/lib/chat'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Dialog, PromptDialog } from '@/components/ui/AppDialog'
import { RowMenu } from '@/components/ui/RowMenu'
import { ChatView } from '@/components/assistant/ChatView'

interface Props {
  workId: string
}

/**
 * The assistant tab of a work's card: this work's chats, and the open one.
 *
 * Talks to Claude through the user's own installed CLI — their subscription,
 * their session. The rest of kilna works without it, so an absent CLI is a
 * message here, not a broken app.
 *
 * A work can carry several chats — one per question worth keeping apart — and
 * none until something is actually asked: the first message creates the chat.
 */
export function AssistantPanel({ workId }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()

  const status = useQuery({
    queryKey: keys.assistantStatus,
    queryFn: assistantStatus,
    // Installing the CLI mid-session is rare; asking once a minute is plenty.
    staleTime: 60_000,
  })

  const summaries = useQuery({
    queryKey: keys.chats(workId),
    queryFn: () => listChatSummaries(workId),
  })

  const active = useQuery({
    queryKey: keys.activeRuns,
    queryFn: activeRuns,
    staleTime: 0,
  })

  // Which chats are running changes on run boundaries, not on every block of
  // an answer — only those events are worth a refetch.
  useEffect(() => {
    const subscription = listen<RunEmission>('assistant:run', ({ payload }) => {
      if (
        payload.event.kind === 'started' ||
        payload.event.kind === 'finished' ||
        payload.event.kind === 'failed' ||
        payload.event.kind === 'stopped'
      ) {
        void client.invalidateQueries({ queryKey: keys.activeRuns })
      }
    })
    return () => {
      void subscription.then((unlisten) => {
        unlisten()
      })
    }
  }, [client])

  const [selected, setSelected] = useState<string | null>(null)
  const [renaming, setRenaming] = useState(false)
  const [confirmingDelete, setConfirmingDelete] = useState(false)

  const chats = summaries.data ?? []
  // The explicit choice, as long as it still exists; the latest chat otherwise.
  const chatId =
    selected !== null && chats.some((chat) => chat.id === selected)
      ? selected
      : (chats[0]?.id ?? null)
  const current = chats.find((chat) => chat.id === chatId)

  const create = useMutation({
    mutationFn: () => createChat({ work_id: workId }),
    onSuccess: (chat) => {
      void client.invalidateQueries({ queryKey: keys.allChats })
      setSelected(chat.id)
    },
    onError: (cause) => {
      say.failed(cause)
    },
  })

  const rename = useMutation({
    mutationFn: ({ id, title }: { id: string; title: string | null }) => renameChat(id, title),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: keys.allChats })
    },
    onError: (cause) => {
      say.failed(cause)
    },
  })

  const remove = useMutation({
    mutationFn: (id: string) => deleteChat(id),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: keys.allChats })
      setSelected(null)
    },
    onError: (cause) => {
      say.failed(cause)
    },
  })

  if (status.data != null && !status.data.available) {
    return (
      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-semibold">{t('assistant.title')}</h3>
        <p className="rounded-xl border border-dashed border-line p-4 text-sm text-dim">
          {status.data.reason ?? t('assistant.unavailable')}
        </p>
      </section>
    )
  }

  const runningChats = new Set(active.data ?? [])

  return (
    <section className="flex flex-col gap-3">
      <div className="flex items-center gap-3">
        <h3 className="text-sm font-semibold">{t('assistant.title')}</h3>
        {status.data?.version != null && (
          <span className="text-xs text-dim">{status.data.version}</span>
        )}
        {current !== undefined && current.cost_usd > 0 && (
          <span className="ml-auto text-xs text-faint">
            {t('assistant.spent', { amount: `$${current.cost_usd.toFixed(2)}` })}
          </span>
        )}
      </div>

      {chats.length > 0 && (
        <div className="flex flex-wrap items-center gap-1.5" role="tablist" aria-label={t('assistant.chats')}>
          {chats.map((chat) => (
            <button
              key={chat.id}
              type="button"
              role="tab"
              aria-selected={chat.id === chatId}
              onClick={() => {
                setSelected(chat.id)
              }}
              className={cn(
                'flex max-w-48 cursor-pointer items-center gap-1.5 rounded-full border px-2.5 py-1 text-xs transition-colors',
                chat.id === chatId
                  ? 'border-accent bg-accent-soft text-accent-2'
                  : 'border-line text-dim hover:border-line-2 hover:text-text',
              )}
            >
              {runningChats.has(chat.id) && (
                <span aria-hidden className="size-1.5 shrink-0 animate-pulse rounded-full bg-accent" />
              )}
              {/* A question waiting in a chat you are not looking at is the
                  thing this mark exists for; a run in flight already pulses. */}
              {chat.waiting_since !== undefined && (
                <MessageCircleQuestion
                  aria-label={t('assistant.waitingMark')}
                  className="size-3.5 shrink-0 text-accent-2"
                />
              )}
              <span className="truncate">{chatLabel(chat, t('assistant.untitled'))}</span>
            </button>
          ))}

          <Button
            variant="icon"
            size="icon-sm"
            title={t('assistant.newChat')}
            aria-label={t('assistant.newChat')}
            disabled={create.isPending}
            onClick={() => {
              create.mutate()
            }}
          >
            <Plus aria-hidden />
          </Button>

          {current !== undefined && (
            <RowMenu
              label={t('assistant.chatMenu')}
              actions={[
                {
                  key: 'rename',
                  label: t('assistant.rename'),
                  onSelect: () => {
                    setRenaming(true)
                  },
                },
                {
                  key: 'delete',
                  label: t('assistant.delete'),
                  danger: true,
                  onSelect: () => {
                    setConfirmingDelete(true)
                  },
                },
              ]}
            />
          )}
        </div>
      )}

      <ChatView
        // A remounted conversation starts scrolled to its end; without the key
        // the list keeps the previous chat's scroll position.
        key={chatId ?? 'empty'}
        chatId={chatId}
        workId={workId}
        onChatCreated={(created) => {
          setSelected(created)
        }}
      />

      <PromptDialog
        open={renaming}
        onOpenChange={setRenaming}
        title={t('assistant.renameTitle')}
        label={t('assistant.renameLabel')}
        initialValue={current?.title ?? ''}
        confirmLabel={t('dialog.save')}
        onSubmit={(value) => {
          if (chatId !== null) rename.mutate({ id: chatId, title: value })
        }}
      />

      <Dialog
        open={confirmingDelete}
        onOpenChange={setConfirmingDelete}
        title={t('assistant.deleteTitle')}
        description={t('assistant.deleteBody')}
        footer={
          <Button
            variant="danger"
            disabled={remove.isPending}
            onClick={() => {
              if (chatId !== null) remove.mutate(chatId)
              setConfirmingDelete(false)
            }}
          >
            {t('assistant.delete')}
          </Button>
        }
      />
    </section>
  )
}
