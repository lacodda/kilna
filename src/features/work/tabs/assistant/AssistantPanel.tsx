import { ChatSurface } from '@/features/assistant/ChatSurface'

interface Props {
  workId: string
}

/**
 * The assistant tab of a work's card: this work's chats as chips across the
 * top, the open one under them, the composer at the foot.
 *
 * The same surface the drawer from the window's bar draws (`ChatSurface`),
 * with the list across the top rather than down the side: a work has a few
 * chats, and the conversation wants the card's width.
 */
export function AssistantPanel({ workId }: Props) {
  return <ChatSurface workId={workId} list="header" />
}
