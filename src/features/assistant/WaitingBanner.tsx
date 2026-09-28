import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { MessageCircleQuestion, X } from 'lucide-react'
import { clearWaiting } from '@/lib/api/assistant'
import type { ChatSummary } from '@/lib/api/types'
import { chatLabel } from '@/lib/chat'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { useAssistant } from '@/lib/useAssistant'
import { Button } from '@/components/ui/button'

/**
 * A background task stopped to ask something, and nobody was there to hear it.
 *
 * This is the other half of a task being fire-and-forget. Walking away is the
 * point — but a question left in a chat nobody opens holds up the work it was
 * about, silently, which is the failure this exists to prevent.
 *
 * Deliberately a banner rather than a toast: a toast is for something that
 * already happened and needs no decision, and this needs one. It stays until
 * the question is answered or dismissed.
 */
export function WaitingBanner() {
  const { t } = useTranslation()
  const assistant = useAssistant()

  // A task can start waiting while this banner is on screen; the run events'
  // bridge refreshes the list on every run's start and end.
  const waiting = useQuery({ ...queries.waitingChats(), staleTime: 0 })

  const dismiss = useAppMutation({
    mutationFn: (chatId: string) => clearWaiting(chatId),
    refresh: [keys.waitingChats, keys.allChats],
  })

  const chats = waiting.data ?? []
  if (chats.length === 0) return null

  // The oldest question — the backend orders by when it was asked, and the one
  // that has been sitting longest is the one holding work up. The rest are
  // counted, not listed: a stack of banners is a wall.
  const [first, ...rest] = chats as [ChatSummary, ...ChatSummary[]]

  return (
    <div
      role="status"
      className="flex items-center gap-3 rounded-xl border border-accent bg-accent-soft px-3 py-2"
    >
      <MessageCircleQuestion aria-hidden className="size-4 shrink-0 text-accent-2" />

      <p className="min-w-0 flex-1 text-sm">
        <span className="text-accent-2">{t('assistant.waitingTitle')}</span>{' '}
        <Button
          variant="link"
          className="max-w-full"
          onClick={() => {
            assistant.open(first.id)
          }}
        >
          <span className="truncate">{chatLabel(first, t('assistant.untitled'))}</span>
        </Button>
        {rest.length > 0 && (
          <span className="text-dim"> {t('assistant.waitingMore', { count: rest.length })}</span>
        )}
      </p>

      <Button
        size="sm"
        onClick={() => {
          assistant.open(first.id)
        }}
      >
        {t('assistant.waitingOpen')}
      </Button>

      <Button
        variant="icon"
        size="icon-sm"
        title={t('assistant.waitingDismiss')}
        aria-label={t('assistant.waitingDismiss')}
        disabled={dismiss.isPending}
        onClick={() => {
          dismiss.mutate(first.id)
        }}
      >
        <X aria-hidden className="size-4" />
      </Button>
    </div>
  )
}
