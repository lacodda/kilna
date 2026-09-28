import { useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { shownChat } from '@/lib/chat'
import { formatCost } from '@/lib/format'
import { EmptyState } from '@/components/ui/empty-state'
import { Frame, ListDetail } from '@/components/frame'
import { ActionBar } from '@/features/assistant/ActionBar'
import { ChatChips, ChatRows, type ChatListProps } from '@/features/assistant/ChatList'
import { ChatView } from '@/features/assistant/ChatView'
import { useChatQuestions, useChats } from '@/features/assistant/chats'

interface Props {
  /** The work whose chats these are. Absent: every chat of the profile. */
  workId?: string
  /**
   * How the chats are drawn beside the conversation. `header` - chips across
   * the top, the card's tab as the mockup draws it (`#p-asst`). `side` - rows
   * down the side, for the drawer, which holds every chat of the profile and
   * needs the room to say what each is about.
   */
  list: 'header' | 'side'
  /** A chat to open on, rather than the latest. */
  initialChat?: string | null
  /** The heading, in place of the caption: the drawer names itself with it. */
  title?: ReactNode
  /** At the far end of the head: the drawer's close button. */
  end?: ReactNode
  /** Go to the card of a chat's work. Where it is given, a chat about a work
   * offers the way there in its menu. */
  onOpenWork?: (workId: string) => void
}

/**
 * The assistant, as one surface: a head saying which CLI answers and what the
 * open chat has cost, the chats, and the open conversation.
 *
 * The drawer from the window's bar and the card's Assistant tab are this
 * component with the list drawn two ways. Until v0.81 each laid out its own
 * list, its own rename and delete, and its own frame around `ChatView` -
 * about a hundred and fifty lines written twice, which had already drifted
 * apart: the drawer hid the conversation behind a back arrow, the tab could
 * not reach a chat about another work.
 *
 * Talks to Claude through the user's own installed CLI — their subscription,
 * their session. The rest of kilna works without it, so an absent CLI is a
 * message here, not a broken app.
 *
 * A work can carry several chats — one per question worth keeping apart — and
 * none until something is actually asked: the first message creates the chat.
 */
export function ChatSurface({ workId, list, initialChat = null, title, end, onOpenWork }: Props) {
  const { t } = useTranslation()
  // The chat chosen by hand, and the one this surface made last - the list is
  // refetched after the write, and until then only this knows it exists. A
  // chat named by the opening (a toast's "open") is trusted the same way.
  const [chosen, setChosen] = useState<string | null>(initialChat)
  const [made, setMade] = useState<string | null>(initialChat)
  const chats = useChats(workId, {
    onCreated: (chat) => {
      setMade(chat.id)
      setChosen(chat.id)
    },
    onRemoved: (id) => {
      if (chosen === id) setChosen(null)
      if (made === id) setMade(null)
    },
  })
  const { actionsOf, dialogs } = useChatQuestions(chats, { onOpenWork })

  const open = shownChat(chats.chats, chosen, made)
  const current = chats.chats.find((chat) => chat.id === open)

  const reason = chats.status?.available === false ? (chats.status.reason ?? null) : undefined
  // With nothing to read, a composer inviting a question the machine cannot
  // answer would be the wrong thing to show: the absence is the whole screen.
  const absent = reason !== undefined && !chats.pending && chats.chats.length === 0

  const head = (
    <>
      {title ?? <h2 className="caption">{t('assistant.title')}</h2>}
      {chats.status?.version != null && (
        <span className="font-mono text-xs text-faint">{chats.status.version}</span>
      )}
      {/* The chats already had are still worth reading without the CLI -
          the drawer always let them be - so its absence is said here, over
          them, rather than in place of them. */}
      {reason !== undefined && !absent && (
        <span className="text-xs text-warn">{reason ?? t('assistant.unavailable')}</span>
      )}
      <span className="ml-auto flex items-center gap-2">
        {current !== undefined && current.cost_usd > 0 && (
          <span className="text-xs text-faint">
            {t('assistant.spent', { amount: formatCost(current.cost_usd) })}
          </span>
        )}
        {end}
      </span>
    </>
  )

  if (absent) {
    return (
      <Frame head={head}>
        <EmptyState
          variant="error"
          title={t('assistant.unavailable')}
          body={reason ?? undefined}
          className="flex-1"
        />
      </Frame>
    )
  }

  const listProps: ChatListProps = {
    chats: chats.chats,
    pending: chats.pending,
    running: chats.running,
    open,
    onOpen: setChosen,
    onCreate: () => {
      chats.create.mutate()
    },
    creating: chats.create.isPending,
    actionsOf,
  }

  const conversation = (
    <ChatView
      // A remounted conversation starts scrolled to its end; without the key
      // the list keeps the previous chat's scroll position.
      key={open ?? 'empty'}
      chatId={open}
      // A chat names its own work in the drawer; the tab's chats are all
      // about the tab's.
      workId={workId ?? current?.work_id ?? undefined}
      onChatCreated={(created) => {
        setMade(created)
        setChosen(created)
      }}
    />
  )

  return (
    <Frame head={head}>
      {list === 'header' ? (
        <div className="flex min-h-0 flex-1 flex-col gap-2.5">
          <ChatChips
            {...listProps}
            // The card's own AI actions, as the mockup places them: each
            // starts a task in a chat of its own, which appears among the
            // chips with its run in flight.
            aside={workId === undefined ? undefined : <ActionBar workId={workId} menu />}
          />
          {conversation}
        </div>
      ) : (
        <ListDetail list={<ChatRows {...listProps} />} detail={conversation} />
      )}
      {dialogs}
    </Frame>
  )
}
