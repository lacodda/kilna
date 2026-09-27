import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { ArrowLeft, ArrowUpRight, MessageSquare, Plus, X } from 'lucide-react'
import type { ChatSummary } from '@/lib/api/types'
import { chatLabel } from '@/lib/chat'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useRunEvent } from '@/lib/runEvents'
import { AssistantContext, useAssistant, type Assistant } from '@/lib/useAssistant'
import { announcement, movesTaskList } from '@/lib/tasks'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { Drawer as DrawerRoot, DrawerClose, DrawerPopup, DrawerTitle } from '@/components/ui/drawer'
import { EmptyState } from '@/components/EmptyState'
import { RowContextMenu, RowMenu, type RowAction } from '@/components/RowMenu'
import { ChatMarks, useChatQuestions, useChats } from '@/features/assistant/chats'
import { ChatView } from '@/features/assistant/ChatView'

/**
 * The assistant from anywhere: a drawer with every chat of the profile, opened
 * by the button in the title bar (`AssistantButton`), which carries a badge
 * for runs in flight.
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

  // Kept current by the run events' bridge on every run's start and end.
  const active = useQuery({ ...queries.activeRuns(), staleTime: 0 })

  // The launcher is always mounted, so this is the one listener that keeps the
  // badge honest wherever the run was started from — and the one place that can
  // announce a finished task from any screen. Only run boundaries matter; not
  // every block of an answer.
  useRunEvent((payload) => {
    if (!movesTaskList(payload)) return

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

  const running = active.data?.length ?? 0

  // Stable, so a consumer re-rendering on every keystroke does not re-subscribe
  // to anything downstream of it.
  const assistant = useMemo<Assistant>(
    () => ({
      open: (chatId?: string) => {
        setOpen({ chatId: chatId ?? null })
      },
      running,
    }),
    [running],
  )

  return (
    <AssistantContext value={assistant}>
      {children}

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
  const navigate = useNavigate()

  const [selected, setSelected] = useState<string | null>(initialChat)
  const shared = useChats(undefined, {
    onCreated: (chat) => setSelected(chat.id),
    onRemoved: (id) => {
      if (selected === id) setSelected(null)
    },
  })
  const { ask, dialogs } = useChatQuestions(shared)
  const chats = shared.chats
  const current = chats.find((chat) => chat.id === selected)

  return (
    <DrawerRoot
      open
      swipeDirection="right"
      onOpenChange={(next) => {
        if (!next) onClose()
      }}
    >
      {/* A fixed header over a scrolling body, the drawer's own anatomy - the
          header here carries the back arrow and the work link as well. */}
      <DrawerPopup className="w-[min(28rem,100vw)] bg-bg">
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
          <DrawerTitle className="truncate text-sm font-semibold">
            {current === undefined
              ? t('assistant.title')
              : chatLabel(current, t('assistant.untitled'))}
          </DrawerTitle>

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

          <DrawerClose
            render={
              <Button
                className="ml-auto"
                variant="icon"
                size="icon-sm"
                aria-label={t('dialog.close')}
              />
            }
          >
            <X aria-hidden />
          </DrawerClose>
        </div>

        {current === undefined ? (
          <div className="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto p-4">
            {shared.status != null && !shared.status.available && (
              <p className="rounded-xl border border-dashed border-line p-3 text-sm text-dim">
                {shared.status.reason ?? t('assistant.unavailable')}
              </p>
            )}

            <Button
              size="sm"
              className="self-start"
              disabled={shared.create.isPending}
              onClick={() => {
                shared.create.mutate()
              }}
            >
              <Plus aria-hidden className="size-3.5" />
              {t('assistant.newChat')}
            </Button>

            {chats.length === 0 && !shared.pending && (
              <EmptyState title={t('assistant.noChatsTitle')} body={t('assistant.noChats')} />
            )}

            <ul className="flex flex-col gap-1">
              {chats.map((chat) => {
                // One list for both ways in: the three dots and the right click.
                const actions: RowAction[] = [
                  {
                    key: 'rename',
                    label: t('assistant.rename'),
                    onSelect: () => {
                      ask.rename(chat)
                    },
                  },
                  {
                    key: 'delete',
                    label: t('assistant.delete'),
                    danger: true,
                    onSelect: () => {
                      ask.remove(chat.id)
                    },
                  },
                ]

                return (
                  <RowContextMenu
                    key={chat.id}
                    actions={actions}
                    render={
                      <li className="flex items-center gap-1 rounded-md data-[popup-open]:bg-soft" />
                    }
                  >
                    <button
                      type="button"
                      onClick={() => {
                        setSelected(chat.id)
                      }}
                      className="flex min-w-0 flex-1 cursor-pointer flex-col gap-0.5 rounded-md px-2.5 py-2 text-left transition-colors hover:bg-soft"
                    >
                      <span className="flex items-center gap-1.5 text-sm">
                        <ChatMarks chat={chat} running={shared.running.has(chat.id)} />
                        <span className="truncate">{chatLabel(chat, t('assistant.untitled'))}</span>
                      </span>
                      <span className="flex items-center gap-2 text-xs text-faint">
                        {chat.work_title != null && (
                          <span className="truncate">{chat.work_title}</span>
                        )}
                        {chat.cost_usd > 0 && <span>${chat.cost_usd.toFixed(2)}</span>}
                      </span>
                    </button>
                    <RowMenu label={t('assistant.chatMenu')} actions={actions} />
                  </RowContextMenu>
                )
              })}
            </ul>
          </div>
        ) : (
          <div className="min-h-0 flex-1 overflow-y-auto p-4">
            <ChatView key={current.id} chatId={current.id} workId={current.work_id ?? undefined} />
          </div>
        )}

        {dialogs}
      </DrawerPopup>
    </DrawerRoot>
  )
}

/**
 * The button that opens the drawer, in the title bar beside the bell.
 *
 * It floated over the bottom right corner of the window until v0.75.2. Once
 * every screen reached the bottom edge instead of scrolling past it, that
 * corner was where the footers are - the score's Record button sat under it.
 * The title bar is the one strip that is the same on every screen, and a
 * control that belongs to no screen belongs there.
 */
export function AssistantButton() {
  const { t } = useTranslation()
  const { open, running } = useAssistant()
  const label = running > 0 ? t('assistant.openBusy') : t('assistant.open')

  return (
    <Button
      variant="icon"
      size="icon-sm"
      className="relative"
      aria-label={label}
      title={label}
      onClick={() => {
        open()
      }}
    >
      <MessageSquare aria-hidden />
      {running > 0 && (
        <span
          aria-hidden
          className="absolute -right-0.5 -top-0.5 flex size-3.5 animate-pulse items-center justify-center rounded-full bg-accent text-[9px] font-semibold text-on-accent"
        >
          {running}
        </span>
      )}
    </Button>
  )
}
