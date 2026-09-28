import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Plus } from 'lucide-react'
import type { ChatSummary } from '@/lib/api/types'
import { chatLabel } from '@/lib/chat'
import { formatCost } from '@/lib/format'
import { Button } from '@/components/ui/button'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { EmptyState } from '@/components/ui/empty-state'
import { RowButton } from '@/components/ui/list-row'
import { SkeletonList } from '@/components/ui/skeleton'
import { Pane } from '@/components/frame'
import { RowContextMenu, RowMenu, type RowAction } from '@/components/RowMenu'
import { ChatMarks } from '@/features/assistant/chats'

/** What both drawings of the list are given - the same list, the same acts. */
export interface ChatListProps {
  chats: ChatSummary[]
  /** Still loading: the rows draw a skeleton rather than "no chats". */
  pending: boolean
  /** The chats with a run in flight. */
  running: ReadonlySet<string>
  /** The chat on screen. */
  open: string | null
  onOpen: (chatId: string) => void
  onCreate: () => void
  creating: boolean
  /** A chat's menu: the same list for the three dots and the right click. */
  actionsOf: (chat: ChatSummary) => RowAction[]
  /** Beside the chips, before the open chat's menu: the card's AI actions. */
  aside?: ReactNode
}

/**
 * The chats as chips across the top of the conversation - the card's tab, as
 * the mockup draws it (`#p-asst`): one chip per chat, the open one lit, a new
 * chat beside them, and the open chat's menu at the far end.
 */
export function ChatChips({
  chats,
  running,
  open,
  onOpen,
  onCreate,
  creating,
  actionsOf,
  aside,
}: ChatListProps) {
  const { t } = useTranslation()
  const current = chats.find((chat) => chat.id === open)

  return (
    <div className="flex shrink-0 flex-wrap items-center gap-1.5">
      {chats.length > 0 && (
        // Pressing the open chip again would let go of it in the group, and
        // "no chat" is not a chat to show, so that press is ignored.
        <ChipGroup
          aria-label={t('assistant.chats')}
          value={open === null ? [] : [open]}
          onValueChange={([next]) => {
            if (next !== undefined) onOpen(next)
          }}
        >
          {chats.map((chat) => (
            <Chip key={chat.id} value={chat.id} className="max-w-48">
              <ChatMarks chat={chat} running={running.has(chat.id)} />
              <span className="truncate">{chatLabel(chat, t('assistant.untitled'))}</span>
            </Chip>
          ))}
        </ChipGroup>
      )}

      <Button variant="ghost" size="xs" disabled={creating} onClick={onCreate}>
        <Plus aria-hidden />
        {t('assistant.newChat')}
      </Button>

      <span className="ml-auto flex items-center gap-1.5">
        {aside}
        {current !== undefined && (
          <RowMenu label={t('assistant.chatMenu')} actions={actionsOf(current)} />
        )}
      </span>
    </div>
  )
}

/**
 * The chats as rows down the side of the conversation - the drawer, which
 * holds every chat of the profile and so needs the room a chip row does not
 * have: each row says what its chat is about and what it has cost, and the
 * open one stays in view beside it rather than behind a back arrow.
 */
export function ChatRows({
  chats,
  pending,
  running,
  open,
  onOpen,
  onCreate,
  creating,
  actionsOf,
}: ChatListProps) {
  const { t } = useTranslation()

  return (
    <Pane
      label={t('assistant.chats')}
      bodyClassName="p-1.5"
      head={
        <>
          <span className="caption">{t('assistant.chats')}</span>
          <Button
            variant="icon"
            size="icon-sm"
            className="ml-auto"
            title={t('assistant.newChat')}
            aria-label={t('assistant.newChat')}
            disabled={creating}
            onClick={onCreate}
          >
            <Plus aria-hidden />
          </Button>
        </>
      }
    >
      {pending ? (
        <SkeletonList rows={3} />
      ) : chats.length === 0 ? (
        <EmptyState plain title={t('assistant.noChatsTitle')} body={t('assistant.noChats')} />
      ) : (
        <ul className="flex flex-col gap-0.5">
          {chats.map((chat) => {
            const actions = actionsOf(chat)
            const busy = running.has(chat.id)
            const waiting = chat.waiting_since !== undefined
            // What the chat is about and what it has cost, under its name.
            const about = [
              chat.work_title ?? '',
              chat.cost_usd > 0 ? formatCost(chat.cost_usd) : '',
            ]
              .filter((part) => part !== '')
              .join(' · ')

            return (
              <RowContextMenu
                key={chat.id}
                actions={actions}
                render={
                  <li className="flex items-center gap-1 rounded-md data-[popup-open]:bg-soft" />
                }
              >
                <RowButton
                  onClick={() => {
                    onOpen(chat.id)
                  }}
                  selected={chat.id === open}
                  className="min-w-0 flex-1"
                  // The marks only when there is one: an empty slot would
                  // still take its gap and push the title off the others.
                  start={
                    busy || waiting ? (
                      <span className="flex items-center gap-1.5">
                        <ChatMarks chat={chat} running={busy} />
                      </span>
                    ) : undefined
                  }
                  description={about === '' ? undefined : about}
                >
                  {chatLabel(chat, t('assistant.untitled'))}
                </RowButton>
                <RowMenu label={t('assistant.chatMenu')} actions={actions} />
              </RowContextMenu>
            )
          })}
        </ul>
      )}
    </Pane>
  )
}
