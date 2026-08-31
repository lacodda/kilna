import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { listen } from '@tauri-apps/api/event'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { MessageCircleQuestion, X } from 'lucide-react'
import { clearWaiting, waitingChats, type ChatSummary, type RunEmission } from '@/lib/api'
import { chatLabel } from '@/lib/chat'
import { keys } from '@/lib/query'
import { movesTaskList } from '@/lib/tasks'
import { say } from '@/lib/toast'
import { useAssistant } from '@/lib/useAssistant'
import { Button } from '@/components/ui/Button'

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
  const client = useQueryClient()
  const assistant = useAssistant()

  const waiting = useQuery({
    queryKey: keys.waitingChats,
    queryFn: waitingChats,
    staleTime: 0,
  })

  // A task can start waiting while this banner is on screen, so run boundaries
  // refresh it. Nothing else does: an answer arriving block by block cannot
  // change whether a question is pending.
  useEffect(() => {
    const subscription = listen<RunEmission>('assistant:run', ({ payload }) => {
      if (movesTaskList(payload)) {
        void client.invalidateQueries({ queryKey: keys.waitingChats })
      }
    })
    return () => {
      void subscription.then((unlisten) => {
        unlisten()
      })
    }
  }, [client])

  const dismiss = useMutation({
    mutationFn: (chatId: string) => clearWaiting(chatId),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: keys.waitingChats })
      void client.invalidateQueries({ queryKey: keys.allChats })
    },
    onError: (cause) => {
      say.failed(cause)
    },
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
        <button
          type="button"
          className="cursor-pointer truncate underline underline-offset-2 hover:text-accent-2"
          onClick={() => {
            assistant.open(first.id)
          }}
        >
          {chatLabel(first, t('assistant.untitled'))}
        </button>
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
