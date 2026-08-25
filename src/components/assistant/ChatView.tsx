import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { listen } from '@tauri-apps/api/event'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import {
  cancelRun,
  createChat,
  getTranscript,
  listRuns,
  renderPrompt,
  startRun,
  type Run,
  type RunEmission,
} from '@/lib/api'
import { conversation, type Exchange } from '@/lib/chat'
import { withEvent } from '@/lib/runs'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/Button'
import { Textarea } from '@/components/ui/Input'
import { Markdown } from '@/components/ui/Markdown'
import { Skeleton } from '@/components/ui/Skeleton'

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
 */
export function ChatView({ chatId, workId, onChatCreated }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const client = useQueryClient()

  const [draft, setDraft] = useState('')
  const bottom = useRef<HTMLDivElement>(null)
  const composer = useRef<HTMLTextAreaElement>(null)

  const transcript = useQuery({
    queryKey: keys.transcript(chatId ?? ''),
    queryFn: () => getTranscript(chatId!),
    enabled: chatId !== null,
  })

  const runs = useQuery({
    queryKey: keys.runs(chatId ?? ''),
    queryFn: () => listRuns(chatId!),
    enabled: chatId !== null,
    // Events keep this fresh while the panel is open; the fetch is for coming
    // back to a chat that was running while the panel was elsewhere.
    staleTime: 0,
  })

  // Events land straight in the cache, so a run reads the same whether its
  // events arrived live or were replayed from storage.
  useEffect(() => {
    if (chatId === null) return

    const subscription = listen<RunEmission>('assistant:run', ({ payload }) => {
      if (payload.chat_id !== chatId) return

      client.setQueryData<Run[]>(keys.runs(chatId), (previous) => {
        if (previous === undefined) return previous
        return previous.map((run) =>
          run.id === payload.run_id ? withEvent(run, payload.event) : run,
        )
      })

      // A finished run wrote an answer; the settled message replaces the
      // overlay's live body, and the chat list's caption and price moved.
      if (payload.event.kind === 'finished' || payload.event.kind === 'failed') {
        void client.invalidateQueries({ queryKey: keys.transcript(chatId) })
        void client.invalidateQueries({ queryKey: keys.allChats })
      }
    })

    return () => {
      void subscription.then((unlisten) => {
        unlisten()
      })
    }
  }, [chatId, client])

  const ask = useMutation({
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
    onSuccess: (run) => {
      setDraft('')
      client.setQueryData<Run[]>(keys.runs(run.chat_id), (previous) => [
        ...(previous ?? []),
        run,
      ])
      void client.invalidateQueries({ queryKey: keys.transcript(run.chat_id) })
      void client.invalidateQueries({ queryKey: keys.allChats })
      if (run.chat_id !== chatId) onChatCreated?.(run.chat_id)
    },
    onError: (cause) => {
      say.failed(cause)
    },
  })

  const stop = useMutation({
    mutationFn: (id: string) => cancelRun(id),
    onError: (cause) => {
      say.failed(cause)
    },
  })

  // A profile action renders its template into the composer rather than
  // firing: what is about to be sent — and paid for — is read and edited
  // first. Enter still does the sending.
  const runTemplate = useMutation({
    mutationFn: (template: string) => renderPrompt(workId!, template),
    onSuccess: (prompt) => {
      setDraft(prompt)
      composer.current?.focus()
      composer.current?.setSelectionRange(0, 0)
      composer.current?.scrollTo({ top: 0 })
    },
    onError: (cause) => {
      say.failed(cause)
    },
  })

  const items = conversation(transcript.data?.messages ?? [], runs.data ?? [])
  const working = items.some((item) => item.run?.working === true)
  const sending = ask.isPending || runTemplate.isPending

  useEffect(() => {
    bottom.current?.scrollIntoView({ block: 'nearest' })
  }, [items.length, working])

  const send = (prompt: string) => {
    if (prompt.trim() === '' || sending) return
    ask.mutate(prompt)
  }

  const copy = (body: string) => {
    void navigator.clipboard.writeText(body).then(() => {
      say.ok(t('assistant.copied'))
    })
  }

  const loading = chatId !== null && (transcript.isPending || runs.isPending)

  return (
    <div className="flex flex-col gap-3">
      {working && <span className="text-xs text-dim">{t('assistant.thinking')}</span>}

      {workId !== undefined && profile.config.prompts.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {profile.config.prompts.map((prompt) => (
            <Button
              key={prompt.key}
              size="sm"
              title={prompt.description}
              disabled={sending}
              onClick={() => {
                runTemplate.mutate(prompt.template)
              }}
            >
              {prompt.label}
            </Button>
          ))}
        </div>
      )}

      {loading && <Skeleton className="h-20 w-full" />}

      {items.length > 0 && (
        <ul className="flex max-h-96 flex-col gap-3 overflow-y-auto">
          {items.map((item) => (
            <ExchangeItem
              key={item.key}
              item={item}
              onCopy={copy}
              onStop={(id) => {
                stop.mutate(id)
              }}
              stopping={stop.isPending}
            />
          ))}
          <div ref={bottom} />
        </ul>
      )}

      <form
        className="flex flex-col gap-2"
        onSubmit={(event) => {
          event.preventDefault()
          send(draft)
        }}
      >
        <Textarea
          ref={composer}
          // A rendered template can be pages long; the box follows it up to a
          // point instead of showing three lines of something worth reading.
          rows={Math.min(10, Math.max(3, draft.split('\n').length))}
          value={draft}
          onChange={(event) => {
            setDraft(event.target.value)
          }}
          placeholder={t(workId === undefined ? 'assistant.placeholderAnywhere' : 'assistant.placeholder')}
          aria-label={t(workId === undefined ? 'assistant.placeholderAnywhere' : 'assistant.placeholder')}
          onKeyDown={(event) => {
            // Enter sends; the panel is for questions, not for composing.
            if (event.key === 'Enter' && !event.shiftKey) {
              event.preventDefault()
              send(draft)
            }
          }}
        />
        <div className="flex justify-end">
          <Button
            type="submit"
            variant="primary"
            disabled={sending || draft.trim() === ''}
          >
            {t('assistant.send')}
          </Button>
        </div>
      </form>
    </div>
  )
}

function ExchangeItem({
  item,
  onCopy,
  onStop,
  stopping,
}: {
  item: Exchange
  onCopy: (body: string) => void
  onStop: (runId: string) => void
  stopping: boolean
}) {
  const { t } = useTranslation()
  const run = item.run

  // The settled message is canonical; a run's own body stands in while it is
  // still growing, or when the run ended before an answer was stored.
  const body = item.answer?.body ?? run?.body ?? ''
  const cost = item.answer?.cost ?? run?.cost ?? null

  return (
    <li className="flex flex-col gap-1.5">
      {item.prompt !== null && (
        <p className="rounded-xl bg-soft px-3 py-2 text-sm whitespace-pre-wrap">{item.prompt}</p>
      )}

      {run !== null && run.steps.length > 0 && (
        <ul className="flex flex-col gap-0.5 px-3 text-xs text-dim">
          {run.steps.map((step, index) => (
            <li key={`${item.key}-${String(index)}`} className="truncate">
              · {step}
            </li>
          ))}
        </ul>
      )}

      {body !== '' && (
        <div className="rounded-xl border border-line px-3 py-2">
          <Markdown body={body} copyLabel={t('assistant.copy')} />
          <div className="mt-1.5 flex items-center gap-2">
            {cost != null && <span className="text-xs text-dim">${cost.toFixed(3)}</span>}
            <Button
              size="sm"
              variant="icon"
              className="ml-auto h-6 px-1.5 text-[11px]"
              onClick={() => {
                onCopy(body)
              }}
            >
              {t('assistant.copy')}
            </Button>
          </div>
        </div>
      )}

      {run?.cancelled === true && (
        <p className="px-3 text-xs text-dim">{t('assistant.stopped')}</p>
      )}

      {run?.failure != null && <p className="px-3 text-xs text-dim">{run.failure}</p>}

      {run?.working === true && (
        <div className="flex items-center gap-2 px-3">
          <span className="text-xs text-dim">{t('assistant.working')}</span>
          <Button
            size="sm"
            disabled={stopping}
            onClick={() => {
              onStop(run.id)
            }}
          >
            {t('assistant.cancel')}
          </Button>
        </div>
      )}
    </li>
  )
}
