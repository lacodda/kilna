import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Plus } from 'lucide-react'
import { chatLabel } from '@/lib/chat'
import { formatCost } from '@/lib/format'
import { Button } from '@/components/ui/button'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { EmptyState } from '@/components/ui/empty-state'
import { Frame } from '@/components/frame'
import { RowMenu } from '@/components/RowMenu'
import { ChatMarks, useChatQuestions, useChats } from '@/features/assistant/chats'
import { ChatView } from '@/features/assistant/ChatView'

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
  const [selected, setSelected] = useState<string | null>(null)
  const chats = useChats(workId, {
    onCreated: (chat) => setSelected(chat.id),
    onRemoved: () => setSelected(null),
  })
  const { ask, dialogs } = useChatQuestions(chats)

  const list = chats.chats
  // The explicit choice, as long as it still exists; the latest chat otherwise.
  const chatId =
    selected !== null && list.some((chat) => chat.id === selected)
      ? selected
      : (list[0]?.id ?? null)
  const current = list.find((chat) => chat.id === chatId)

  if (chats.status != null && !chats.status.available) {
    return (
      <Frame>
        <EmptyState
          variant="error"
          title={t('assistant.unavailable')}
          body={chats.status.reason ?? undefined}
          className="flex-1"
        />
      </Frame>
    )
  }

  return (
    <Frame
      head={
        <>
          {chats.status?.version != null && (
            <span className="text-xs text-dim">{chats.status.version}</span>
          )}
          {current !== undefined && current.cost_usd > 0 && (
            <span className="ml-auto text-xs text-faint">
              {t('assistant.spent', { amount: formatCost(current.cost_usd) })}
            </span>
          )}
        </>
      }
    >
      {list.length > 0 && (
        <div className="mb-2.5 flex shrink-0 flex-wrap items-center gap-1.5">
          {/* The chats as a row of chips, one of them open. Pressing the open
              one again would let go of it in the group, and "no chat" is not
              a chat to show, so that press is ignored. */}
          <ChipGroup
            aria-label={t('assistant.chats')}
            value={chatId === null ? [] : [chatId]}
            onValueChange={([next]) => {
              if (next !== undefined) setSelected(next)
            }}
          >
            {list.map((chat) => (
              <Chip key={chat.id} value={chat.id} className="max-w-48">
                <ChatMarks chat={chat} running={chats.running.has(chat.id)} />
                <span className="truncate">{chatLabel(chat, t('assistant.untitled'))}</span>
              </Chip>
            ))}
          </ChipGroup>

          <Button
            variant="icon"
            size="icon-sm"
            title={t('assistant.newChat')}
            aria-label={t('assistant.newChat')}
            disabled={chats.create.isPending}
            onClick={() => {
              chats.create.mutate()
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
                    ask.rename(current)
                  },
                },
                {
                  key: 'delete',
                  label: t('assistant.delete'),
                  danger: true,
                  onSelect: () => {
                    ask.remove(current.id)
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

      {dialogs}
    </Frame>
  )
}
