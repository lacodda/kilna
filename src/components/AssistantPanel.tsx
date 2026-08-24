import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { listen } from '@tauri-apps/api/event'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import {
  assistantStatus,
  cancelRun,
  createChat,
  listChats,
  listRuns,
  renderPrompt,
  startRun,
  type Run,
  type RunEmission,
} from '@/lib/api'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { useProfile } from '@/lib/useProfile'
import { inOrder, view, withEvent } from '@/lib/runs'
import { Button } from '@/components/ui/Button'
import { Textarea } from '@/components/ui/Input'
import { Skeleton } from '@/components/ui/Skeleton'

interface Props {
  workId: string
}

// Talks to Claude through the user's own installed CLI — their subscription,
// their session. The rest of kilna works without it, so an absent CLI is a
// message here, not a broken app.
//
// A run is not held by this component: it belongs to its chat in the backend,
// and what it says arrives as events. Leaving the work and coming back replays
// the run from what was stored, so the panel shows the same thing either way.
export function AssistantPanel({ workId }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const client = useQueryClient()

  const [draft, setDraft] = useState('')
  const bottom = useRef<HTMLDivElement>(null)

  const status = useQuery({
    queryKey: keys.assistantStatus,
    queryFn: assistantStatus,
    // Installing the CLI mid-session is rare; asking once a minute is plenty.
    staleTime: 60_000,
  })

  // One chat per work, reused across sessions. Creating it on demand is part of
  // reading it — the panel has no meaning without one.
  const chat = useQuery({
    queryKey: keys.chats(workId),
    queryFn: async () => {
      const chats = await listChats(workId)
      return chats[0] ?? (await createChat({ work_id: workId }))
    },
  })

  const chatId = chat.data?.id ?? null

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

      // A finished run wrote an answer and may have moved the work on.
      if (payload.event.kind === 'finished' || payload.event.kind === 'failed') {
        void client.invalidateQueries({ queryKey: keys.transcript(chatId) })
      }
    })

    return () => {
      void subscription.then((unlisten) => {
        unlisten()
      })
    }
  }, [chatId, client])

  const ask = useMutation({
    mutationFn: (prompt: string) => startRun(chatId!, prompt),
    // The run comes back the moment the CLI is spawned; putting it in the cache
    // is what makes the question appear at once.
    onSuccess: (run) => {
      setDraft('')
      client.setQueryData<Run[]>(keys.runs(run.chat_id), (previous) => [
        ...(previous ?? []),
        run,
      ])
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

  const runTemplate = useMutation({
    mutationFn: (template: string) => renderPrompt(workId, template),
    onSuccess: (prompt) => {
      ask.mutate(prompt)
    },
    onError: (cause) => {
      say.failed(cause)
    },
  })

  const views = inOrder(runs.data ?? []).map(view)
  const working = views.some((run) => run.working)
  // Sending is blocked only while the request itself is in flight: a run in
  // progress no longer holds the panel, which is the point of this stage.
  const sending = ask.isPending || runTemplate.isPending

  useEffect(() => {
    bottom.current?.scrollIntoView({ block: 'nearest' })
  }, [views.length, working])

  const send = (prompt: string) => {
    if (chatId === null || prompt.trim() === '' || sending) return
    ask.mutate(prompt)
  }

  if (status.data != null && !status.data.available) {
    return (
      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-semibold">{t('assistant.title')}</h3>
        <p className="rounded-xl border border-dashed border-line p-4 text-sm text-dim">
          {status.data.reason ?? t('assistant.unavailable')}
        </p>
      </section>
    )
  }

  return (
    <section className="flex flex-col gap-3">
      <div className="flex items-center gap-3">
        <h3 className="text-sm font-semibold">{t('assistant.title')}</h3>
        {status.data?.version != null && (
          <span className="text-xs text-dim">{status.data.version}</span>
        )}
        {working && <span className="text-xs text-dim">{t('assistant.thinking')}</span>}
      </div>

      {profile.config.prompts.length > 0 && (
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

      {runs.isPending && chatId !== null && <Skeleton className="h-20 w-full" />}

      {views.length > 0 && (
        <ul className="flex max-h-96 flex-col gap-3 overflow-y-auto">
          {views.map((run) => (
            <li key={run.id} className="flex flex-col gap-1.5">
              <p className="rounded-xl bg-soft px-3 py-2 text-sm whitespace-pre-wrap">
                {run.prompt}
              </p>

              {run.steps.length > 0 && (
                <ul className="flex flex-col gap-0.5 px-3 text-xs text-dim">
                  {run.steps.map((step, index) => (
                    <li key={`${run.id}-${String(index)}`} className="truncate">
                      · {step}
                    </li>
                  ))}
                </ul>
              )}

              {run.body !== '' && (
                <div className="rounded-xl border border-line px-3 py-2 text-sm">
                  <p className="whitespace-pre-wrap">{run.body}</p>
                  {run.cost != null && (
                    <p className="mt-1 text-xs text-dim">${run.cost.toFixed(3)}</p>
                  )}
                </div>
              )}

              {run.cancelled && (
                <p className="px-3 text-xs text-dim">{t('assistant.stopped')}</p>
              )}

              {run.failure != null && (
                <p className="px-3 text-xs text-dim">{run.failure}</p>
              )}

              {run.working && (
                <div className="flex items-center gap-2 px-3">
                  <span className="text-xs text-dim">{t('assistant.working')}</span>
                  <Button
                    size="sm"
                    disabled={stop.isPending}
                    onClick={() => {
                      stop.mutate(run.id)
                    }}
                  >
                    {t('assistant.cancel')}
                  </Button>
                </div>
              )}
            </li>
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
          rows={3}
          value={draft}
          onChange={(event) => {
            setDraft(event.target.value)
          }}
          placeholder={t('assistant.placeholder')}
          aria-label={t('assistant.placeholder')}
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
    </section>
  )
}
