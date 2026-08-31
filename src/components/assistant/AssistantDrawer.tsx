import { useEffect, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { listen } from '@tauri-apps/api/event'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import * as Primitive from '@radix-ui/react-dialog'
import {
  ArrowLeft,
  ArrowUpRight,
  MessageCircleQuestion,
  MessageSquare,
  Plus,
  X,
} from 'lucide-react'
import {
  activeRuns,
  assistantStatus,
  createChat,
  deleteChat,
  listChatSummaries,
  renameChat,
  type ChatSummary,
  type RunEmission,
} from '@/lib/api'
import { chatLabel } from '@/lib/chat'
import { keys } from '@/lib/query'
import { AssistantContext, type Assistant } from '@/lib/useAssistant'
import { announcement, movesTaskList } from '@/lib/tasks'
import { say } from '@/lib/toast'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/Button'
import { Dialog, PromptDialog } from '@/components/ui/Dialog'
import { EmptyState } from '@/components/ui/EmptyState'
import { RowMenu } from '@/components/ui/RowMenu'
import { ChatView } from '@/components/assistant/ChatView'

/**
 * The assistant from anywhere: a floating button with a badge for runs in
 * flight, opening a drawer with every chat of the profile.
 *
 * The card's panel shows one work's chats; this is the other half of the
 * promise that a run belongs to its chat — wherever you are, what is running
 * is one click away, and a chat does not need a work to be about.
 */
export function AssistantLauncher({ children }: { children?: React.ReactNode }) {
  const { t } = useTranslation()
  const client = useQueryClient()

  // Null when closed. A string opens the drawer straight onto that chat —
  // what the "open" on a finished task's toast does, so the answer is one
  // click away from wherever the person happened to be.
  const [open, setOpen] = useState<{ chatId: string | null } | null>(null)

  const active = useQuery({
    queryKey: keys.activeRuns,
    queryFn: activeRuns,
    staleTime: 0,
  })

  // The launcher is always mounted, so this is the one listener that keeps the
  // badge honest wherever the run was started from — and the one place that can
  // announce a finished task from any screen. Only run boundaries matter; not
  // every block of an answer.
  useEffect(() => {
    const subscription = listen<RunEmission>('assistant:run', ({ payload }) => {
      if (!movesTaskList(payload)) return

      void client.invalidateQueries({ queryKey: keys.activeRuns })
      void client.invalidateQueries({ queryKey: keys.activeTasks })

      const ending = announcement(payload)
      if (ending === null) return

      // The chat's name, if a list has already been loaded. Worth no fetch of
      // its own: the toast is useful without it.
      const named = client
        .getQueriesData<ChatSummary[]>({ queryKey: keys.allChats })
        .flatMap(([, chats]) => chats ?? [])
        .find((chat) => chat.id === payload.chat_id)
      const what = named === undefined ? null : chatLabel(named, t('assistant.untitled'))

      const message =
        ending === 'done'
          ? what === null
            ? t('assistant.taskDone')
            : t('assistant.taskDoneNamed', { title: what })
          : what === null
            ? t('assistant.taskEnded')
            : t('assistant.taskEndedNamed', { title: what })

      say.withAction(message, t('assistant.taskOpen'), () => {
        setOpen({ chatId: payload.chat_id })
      })
    })
    return () => {
      void subscription.then((unlisten) => {
        unlisten()
      })
    }
  }, [client, t])

  const running = active.data?.length ?? 0

  // Stable, so a consumer re-rendering on every keystroke does not re-subscribe
  // to anything downstream of it.
  const assistant = useMemo<Assistant>(
    () => ({
      open: (chatId?: string) => {
        setOpen({ chatId: chatId ?? null })
      },
    }),
    [],
  )

  return (
    <AssistantContext value={assistant}>
      {children}

      <button
        type="button"
        aria-label={running > 0 ? t('assistant.openBusy') : t('assistant.open')}
        title={running > 0 ? t('assistant.openBusy') : t('assistant.open')}
        onClick={() => {
          setOpen({ chatId: null })
        }}
        className={cn(
          'fixed bottom-5 right-5 z-40 flex size-11 cursor-pointer items-center justify-center rounded-full',
          'border border-line bg-raise text-dim shadow-raise transition-colors hover:text-text',
        )}
      >
        <MessageSquare aria-hidden className="size-5" />
        {running > 0 && (
          <span
            aria-hidden
            className="absolute -right-0.5 -top-0.5 flex size-4.5 animate-pulse items-center justify-center rounded-full bg-accent text-[10px] font-semibold text-on-accent"
          >
            {running}
          </span>
        )}
      </button>

      {open !== null && (
        <Drawer
          initialChat={open.chatId}
          onClose={() => {
            setOpen(null)
          }}
        />
      )}
    </AssistantContext>
  )
}

/** Mounted per opening, so it always starts where the opening asked for. */
function Drawer({
  initialChat,
  onClose,
}: {
  /** A chat to open on, or null for the list. */
  initialChat: string | null
  onClose: () => void
}) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const navigate = useNavigate()

  const [selected, setSelected] = useState<string | null>(initialChat)
  const [renaming, setRenaming] = useState<string | null>(null)
  const [confirmingDelete, setConfirmingDelete] = useState<string | null>(null)

  const status = useQuery({
    queryKey: keys.assistantStatus,
    queryFn: assistantStatus,
    staleTime: 60_000,
  })

  const summaries = useQuery({
    queryKey: keys.chats(),
    queryFn: () => listChatSummaries(),
  })

  const active = useQuery({
    queryKey: keys.activeRuns,
    queryFn: activeRuns,
    staleTime: 0,
  })

  const chats = summaries.data ?? []
  const current = chats.find((chat) => chat.id === selected)
  const runningChats = new Set(active.data ?? [])

  const create = useMutation({
    mutationFn: () => createChat({}),
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
    onSuccess: (_, id) => {
      void client.invalidateQueries({ queryKey: keys.allChats })
      if (selected === id) setSelected(null)
    },
    onError: (cause) => {
      say.failed(cause)
    },
  })

  const renamed = renaming === null ? undefined : chats.find((chat) => chat.id === renaming)

  return (
    <Primitive.Root
      open
      onOpenChange={(next) => {
        if (!next) onClose()
      }}
    >
      <Primitive.Portal>
        <Primitive.Overlay className="fixed inset-0 z-50 bg-black/40" />
        <Primitive.Content
          className="fixed inset-y-0 right-0 z-50 flex w-[min(28rem,100vw)] flex-col border-l border-line bg-bg shadow-raise"
          aria-describedby={undefined}
        >
          <div className="flex items-center gap-2 border-b border-line px-4 py-3">
            {current !== undefined && (
              <Button
                variant="icon"
                size="icon-sm"
                aria-label={t('assistant.back')}
                title={t('assistant.back')}
                onClick={() => {
                  setSelected(null)
                }}
              >
                <ArrowLeft aria-hidden />
              </Button>
            )}
            <Primitive.Title className="truncate text-sm font-semibold">
              {current === undefined
                ? t('assistant.title')
                : chatLabel(current, t('assistant.untitled'))}
            </Primitive.Title>

            {current?.work_id != null && (
              <Button
                variant="icon"
                size="icon-sm"
                aria-label={t('assistant.openWork')}
                title={t('assistant.openWork')}
                onClick={() => {
                  onClose()
                  void navigate(`/works/${current.work_id}/assistant`)
                }}
              >
                <ArrowUpRight aria-hidden />
              </Button>
            )}

            <Primitive.Close asChild>
              <Button
                className="ml-auto"
                variant="icon"
                size="icon-sm"
                aria-label={t('dialog.close')}
              >
                <X aria-hidden />
              </Button>
            </Primitive.Close>
          </div>

          {current === undefined ? (
            <div className="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto p-4">
              {status.data != null && !status.data.available && (
                <p className="rounded-xl border border-dashed border-line p-3 text-sm text-dim">
                  {status.data.reason ?? t('assistant.unavailable')}
                </p>
              )}

              <Button
                size="sm"
                className="self-start"
                disabled={create.isPending}
                onClick={() => {
                  create.mutate()
                }}
              >
                <Plus aria-hidden className="size-3.5" />
                {t('assistant.newChat')}
              </Button>

              {chats.length === 0 && !summaries.isPending && (
                <EmptyState title={t('assistant.noChatsTitle')} body={t('assistant.noChats')} />
              )}

              <ul className="flex flex-col gap-1">
                {chats.map((chat) => (
                  <li key={chat.id} className="flex items-center gap-1">
                    <button
                      type="button"
                      onClick={() => {
                        setSelected(chat.id)
                      }}
                      className="flex min-w-0 flex-1 cursor-pointer flex-col gap-0.5 rounded-[10px] px-2.5 py-2 text-left transition-colors hover:bg-soft"
                    >
                      <span className="flex items-center gap-1.5 text-sm">
                        {runningChats.has(chat.id) && (
                          <span
                            aria-hidden
                            className="size-1.5 shrink-0 animate-pulse rounded-full bg-accent"
                          />
                        )}
                        {chat.waiting_since !== undefined && (
                          <MessageCircleQuestion
                            aria-label={t('assistant.waitingMark')}
                            className="size-3.5 shrink-0 text-accent-2"
                          />
                        )}
                        <span className="truncate">
                          {chatLabel(chat, t('assistant.untitled'))}
                        </span>
                      </span>
                      <span className="flex items-center gap-2 text-xs text-faint">
                        {chat.work_title != null && (
                          <span className="truncate">{chat.work_title}</span>
                        )}
                        {chat.cost_usd > 0 && <span>${chat.cost_usd.toFixed(2)}</span>}
                      </span>
                    </button>
                    <RowMenu
                      label={t('assistant.chatMenu')}
                      actions={[
                        {
                          key: 'rename',
                          label: t('assistant.rename'),
                          onSelect: () => {
                            setRenaming(chat.id)
                          },
                        },
                        {
                          key: 'delete',
                          label: t('assistant.delete'),
                          danger: true,
                          onSelect: () => {
                            setConfirmingDelete(chat.id)
                          },
                        },
                      ]}
                    />
                  </li>
                ))}
              </ul>
            </div>
          ) : (
            <div className="min-h-0 flex-1 overflow-y-auto p-4">
              <ChatView
                key={current.id}
                chatId={current.id}
                workId={current.work_id ?? undefined}
              />
            </div>
          )}

          <PromptDialog
            open={renaming !== null}
            onOpenChange={(next) => {
              if (!next) setRenaming(null)
            }}
            title={t('assistant.renameTitle')}
            label={t('assistant.renameLabel')}
            initialValue={renamed?.title ?? ''}
            confirmLabel={t('dialog.save')}
            onSubmit={(value) => {
              if (renaming !== null) rename.mutate({ id: renaming, title: value })
            }}
          />

          <Dialog
            open={confirmingDelete !== null}
            onOpenChange={(next) => {
              if (!next) setConfirmingDelete(null)
            }}
            title={t('assistant.deleteTitle')}
            description={t('assistant.deleteBody')}
            footer={
              <Button
                variant="danger"
                disabled={remove.isPending}
                onClick={() => {
                  if (confirmingDelete !== null) remove.mutate(confirmingDelete)
                  setConfirmingDelete(null)
                }}
              >
                {t('assistant.delete')}
              </Button>
            }
          />
        </Primitive.Content>
      </Primitive.Portal>
    </Primitive.Root>
  )
}
