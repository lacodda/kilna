import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { applyPendingProposals, cancelRun, createChat, startRun } from '@/lib/api/assistant'
import type { Run } from '@/lib/api/types'
import { conversation, pending } from '@/lib/chat'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { SkeletonList } from '@/components/ui/skeleton'
import { Pane } from '@/components/frame'
import { Composer } from '@/features/assistant/Composer'
import { ExchangeItem } from '@/features/assistant/Exchange'
import { InsertVersionDialog } from '@/features/assistant/InsertVersionDialog'
import { KeepAsNoteDialog } from '@/features/assistant/KeepAsNoteDialog'

interface Props {
  /** Null when the chat does not exist yet — sending the first message creates it. */
  chatId: string | null
  /** The work the chat is about. Absent for a chat about nothing in particular. */
  workId?: string
  /** Told when the first message had to create the chat. */
  onChatCreated?: (chatId: string) => void
}

/**
 * One conversation: what was said, what a run is doing right now, and the
 * composer.
 *
 * The transcript is the spine and runs are an overlay on it — the same
 * conversation reads the same whether its runs are live, replayed, or long
 * settled into messages. A run in flight does not hold this component: it
 * belongs to its chat in the backend, and what it says arrives as events.
 *
 * The mockup's panel (`.panel.chat`): the exchange scrolls, the composer
 * stands at the foot. The exchange was a box of 384 pixels inside a tab that
 * scrolled whole until v0.78, so a long answer was read through a letterbox
 * and the composer went off the card with it.
 */
export function ChatView({ chatId, workId, onChatCreated }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()

  const [draft, setDraft] = useState('')
  const [inserting, setInserting] = useState<{
    body: string
    role?: string
    label?: string
    messageId?: string
  } | null>(null)
  /** The answer being kept as a note, or null. */
  const [keeping, setKeeping] = useState<string | null>(null)
  const bottom = useRef<HTMLDivElement>(null)

  const transcript = useQuery({
    ...queries.transcript(chatId ?? ''),
    enabled: chatId !== null,
    // A proposal from outside the window — `kilna --mcp` — lands in this
    // chat without an event; while the chat is open it appears within a
    // few seconds rather than on the next click.
    refetchInterval: 10_000,
  })

  // The run events' bridge lands every event on its run here, so a run reads
  // the same whether its events arrived live or were replayed from storage;
  // the fetch is for coming back to a chat that was running elsewhere.
  const runs = useQuery({
    ...queries.runs(chatId ?? ''),
    enabled: chatId !== null,
    staleTime: 0,
  })

  const ask = useAppMutation({
    mutationFn: async (prompt: string) => {
      let id = chatId
      // The first message is what brings the chat into being: a chat is a
      // conversation, and an empty one for every card ever opened is noise.
      if (id === null) {
        const chat = await createChat({ work_id: workId ?? null })
        id = chat.id
      }
      return startRun(id, prompt)
    },
    // The run comes back the moment the CLI is spawned; putting it in the
    // cache is what makes the question appear at once.
    // The transcript is the chat the run landed in, which may be the one it
    // just brought into being - hence refreshed by hand, from the run.
    refresh: [keys.allChats],
    onSuccess: (run) => {
      setDraft('')
      client.setQueryData<Run[]>(keys.runs(run.chat_id), (previous) => [...(previous ?? []), run])
      void client.invalidateQueries({ queryKey: keys.transcript(run.chat_id) })
      if (run.chat_id !== chatId) onChatCreated?.(run.chat_id)
    },
  })

  const items = conversation(transcript.data?.messages ?? [], runs.data ?? [])
  const going = items.flatMap((item) => (item.run?.working === true ? [item.run.id] : []))
  const waiting = pending(items)

  // Every run of the chat still going; one, almost always, but nothing stops
  // a second question from being sent while the first is answered.
  const stop = useAppMutation({
    mutationFn: (ids: string[]) => Promise.all(ids.map((id) => cancelRun(id))),
  })

  // Every proposal in the chat, one click: an agent that sends a lyric, a
  // style, a score and two notes as five messages is applied as one package
  // would be. Each still lands as its own operation, so undo takes them back
  // one at a time.
  const applyAll = useMutation({
    mutationFn: () => applyPendingProposals(chatId ?? ''),
    onSuccess: (applied) => {
      say.ok(t('assistant.appliedAll', { count: applied.length }))
    },
    onError: (cause) => {
      say.failed(cause)
    },
    // Whatever was applied — all of it, or the ones before the failure —
    // moved works, versions, scores and notes; one broad refresh.
    onSettled: () => {
      void client.invalidateQueries()
    },
  })

  useEffect(() => {
    bottom.current?.scrollIntoView({ block: 'nearest' })
  }, [items.length, going.length])

  const loading = chatId !== null && (transcript.isPending || runs.isPending)

  return (
    <>
      <Pane
        label={t('assistant.conversation')}
        bodyClassName="flex flex-col gap-3 p-3.5"
        // What waits on the person across the whole chat, over it: one card
        // under each answer says the same thing one at a time.
        head={
          chatId !== null && waiting.length > 1 ? (
            <>
              <span className="text-xs text-dim">
                {t('assistant.pendingCount', { count: waiting.length })}
              </span>
              <Button
                className="ml-auto"
                size="xs"
                variant="primary"
                disabled={applyAll.isPending}
                onClick={() => {
                  applyAll.mutate()
                }}
              >
                {t('assistant.applyAll')}
              </Button>
            </>
          ) : undefined
        }
        foot={
          <Composer
            workId={workId}
            draft={draft}
            onDraft={setDraft}
            onSend={(prompt) => {
              ask.mutate(prompt)
            }}
            sending={ask.isPending}
            working={going.length > 0}
            onStop={() => {
              stop.mutate(going)
            }}
            stopping={stop.isPending}
          />
        }
      >
        {loading ? (
          <SkeletonList rows={2} />
        ) : items.length === 0 ? (
          // Plain, and its way out is the composer under it.
          <EmptyState plain title={t('assistant.nothingAsked')} />
        ) : (
          <ul className="flex flex-col gap-3">
            {items.map((item) => (
              <ExchangeItem
                key={item.key}
                item={item}
                workId={workId}
                onInsert={
                  workId === undefined
                    ? undefined
                    : (body, role, label, messageId) => {
                        setInserting({ body, role, label, messageId })
                      }
                }
                onKeepAsNote={(body) => {
                  setKeeping(body)
                }}
              />
            ))}
          </ul>
        )}
        <div ref={bottom} />
      </Pane>

      {/* A note does not need a work: a chat about nothing still produces
          answers worth keeping, and they become the workspace's notes. */}
      {keeping !== null && (
        <KeepAsNoteDialog
          open
          onOpenChange={(open) => {
            if (!open) setKeeping(null)
          }}
          body={keeping}
          workId={workId}
        />
      )}

      {/* Mounted per opening, so the role and label start fresh each time. */}
      {workId !== undefined && inserting !== null && (
        <InsertVersionDialog
          open
          onOpenChange={(open) => {
            if (!open) setInserting(null)
          }}
          workId={workId}
          body={inserting.body}
          role={inserting.role}
          label={inserting.label}
          messageId={inserting.messageId}
        />
      )}
    </>
  )
}
