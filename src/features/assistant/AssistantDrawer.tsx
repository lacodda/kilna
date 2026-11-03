import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { MessageSquare, X } from 'lucide-react'
import { checkSpend } from '@/lib/api/assistant'
import type { Activity, ChatSummary } from '@/lib/api/types'
import { chatLabel } from '@/lib/chat'
import { endingOf, HOLD_MS, isIdle, sayEndings, type Ending } from '@/lib/endings'
import { inFront, notify, tell } from '@/lib/outward'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useRunEvent } from '@/lib/runEvents'
import { AssistantContext, useAssistant, type Assistant } from '@/lib/useAssistant'
import { announcement, movesTaskList } from '@/lib/tasks'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { Drawer as DrawerRoot, DrawerClose, DrawerPopup, DrawerTitle } from '@/components/ui/drawer'
import { ChatSurface } from '@/features/assistant/ChatSurface'

/**
 * The assistant from anywhere: a drawer with every chat of the profile, opened
 * by the button in the title bar (`AssistantButton`), which carries a badge
 * for runs in flight.
 *
 * The card's tab shows one work's chats; this is the other half of the
 * promise that a run belongs to its chat — wherever you are, what is running
 * is one click away, and a chat does not need a work to be about. Both are
 * one `ChatSurface`.
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
  const busy = useQuery(queries.activity())

  // Endings heard while the window was not in front, waiting to be said as
  // one notification (v0.107, `lib/endings`).
  const held = useRef<Ending[]>([])
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null)
  const flush = useCallback(() => {
    if (timer.current !== null) clearTimeout(timer.current)
    timer.current = null
    const endings = held.current
    held.current = []
    // Back in front before they were said: the window shows them itself.
    if (endings.length === 0 || inFront()) return
    const said = sayEndings(endings, t)
    const chatId = endings.length === 1 ? (endings[0]?.chatId ?? null) : null
    notify({
      ...said,
      open: { label: t('assistant.taskOpen'), run: () => setOpen({ chatId }) },
    })
  }, [t])
  // The last run of a batch is over: say what was held now, not in a minute.
  useEffect(() => {
    if (isIdle(busy.data)) flush()
  }, [busy.data, flush])
  useEffect(
    () => () => {
      if (timer.current !== null) clearTimeout(timer.current)
    },
    [],
  )

  // A run ended, and it was priced: the week may have gone over its budget.
  // Said once, the first time it is seen (`spend::check`).
  const checkBudget = useCallback(() => {
    checkSpend()
      .then(({ spend, crossed }) => {
        if (!crossed) return
        void client.invalidateQueries({ queryKey: keys.journal })
        tell({
          title: t('journal.assistant.overBudget', { spent: spend.week, budget: spend.budget }),
          warn: true,
          open: { label: t('assistant.taskOpen'), run: () => setOpen({ chatId: null }) },
        })
      })
      .catch(() => undefined)
  }, [client, t])

  // The launcher is always mounted, so this is the one listener that keeps the
  // badge honest wherever the run was started from — and the one place that can
  // announce a finished task from any screen. Only run boundaries matter; not
  // every block of an answer.
  useRunEvent((payload) => {
    if (!movesTaskList(payload)) return

    // The chat's name, if a list has already been loaded. Worth no fetch of
    // its own: the toast is useful without it.
    const named = client
      .getQueriesData<ChatSummary[]>({ queryKey: keys.allChats })
      .flatMap(([, chats]) => chats ?? [])
      .find((chat) => chat.id === payload.chat_id)
    const what = named === undefined ? null : chatLabel(named, t('assistant.untitled'))

    const over = endingOf(payload, what)
    if (over !== null) checkBudget()

    // Out of sight, every ending is worth telling - a question typed before
    // the window was closed included - and they are said together.
    if (over !== null && !inFront()) {
      held.current.push(over)
      if (isIdle(client.getQueryData<Activity>(keys.activity))) flush()
      else timer.current ??= setTimeout(flush, HOLD_MS)
      return
    }

    const ending = announcement(payload)
    if (ending === null) return

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

/**
 * The drawer itself: every chat of the profile down the side, the open one
 * beside it. The same surface as the card's tab, with the list drawn as rows.
 *
 * Mounted per opening, so it always starts where the opening asked for.
 */
function Drawer({
  initialChat,
  onClose,
}: {
  /** A chat to open on, or null for the latest. */
  initialChat: string | null
  onClose: () => void
}) {
  const { t } = useTranslation()
  const navigate = useNavigate()

  return (
    <DrawerRoot
      open
      swipeDirection="right"
      onOpenChange={(next) => {
        if (!next) onClose()
      }}
    >
      {/* Wide enough for the list and a conversation beside it: at the width
          of a side panel the chat was a column of three words a line, and the
          list had to be left for it. */}
      <DrawerPopup size="xl" className="bg-bg p-4">
        <ChatSurface
          list="side"
          initialChat={initialChat}
          title={<DrawerTitle className="text-sm">{t('assistant.title')}</DrawerTitle>}
          end={
            <DrawerClose
              render={<Button variant="icon" size="icon-sm" aria-label={t('dialog.close')} />}
            >
              <X aria-hidden />
            </DrawerClose>
          }
          // The drawer goes as the card comes: a work opened behind a modal
          // panel would be a screen nobody can reach until it is closed.
          onOpenWork={(workId) => {
            onClose()
            void navigate(`/works/${workId}/assistant`)
          }}
        />
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
          className="absolute -right-0.5 -top-0.5 flex size-3.5 animate-pulse items-center justify-center rounded-full bg-accent text-2xs font-semibold text-on-accent"
        >
          {running}
        </span>
      )}
    </Button>
  )
}
