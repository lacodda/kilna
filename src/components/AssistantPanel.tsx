import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import {
  askAssistant,
  assistantStatus,
  createChat,
  getTranscript,
  listChats,
  renderPrompt,
  type Message,
} from '@/lib/api'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/Button'
import { Textarea } from '@/components/ui/Input'
import { Skeleton } from '@/components/ui/Skeleton'
import { cn } from '@/lib/utils'

interface Props {
  workId: string
}

// Talks to Claude through the user's own installed CLI — their subscription,
// their session. The rest of kilna works without it, so an absent CLI is a
// message here, not a broken app.
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

  const transcript = useQuery({
    queryKey: keys.transcript(chatId ?? ''),
    queryFn: () => getTranscript(chatId!),
    enabled: chatId !== null,
  })

  const messages = transcript.data?.messages ?? []

  const ask = useMutation({
    mutationFn: (prompt: string) => askAssistant(chatId!, prompt),
    // Show the question immediately — a CLI round trip is seconds long, and a
    // composer that empties into silence looks broken.
    onMutate: (prompt) => {
      const key = keys.transcript(chatId ?? '')
      const previous = client.getQueryData<{ messages: Message[] }>(key)

      if (previous !== undefined) {
        const pending: Message = {
          id: `pending-${String(previous.messages.length)}`,
          chat_id: chatId!,
          role: 'user',
          body: prompt,
          meta: {},
          created_at: new Date().toISOString(),
        }
        client.setQueryData(key, { ...previous, messages: [...previous.messages, pending] })
      }
      setDraft('')
      return { previous }
    },
    onError: (cause, _prompt, context) => {
      // Drop the echoed question: it was never answered, and leaving it there
      // would suggest it was.
      if (context?.previous !== undefined) {
        client.setQueryData(keys.transcript(chatId ?? ''), context.previous)
      }
      say.failed(cause)
    },
    onSettled: () => {
      void client.invalidateQueries({ queryKey: keys.transcript(chatId ?? '') })
    },
  })

  const runTemplate = useMutation({
    mutationFn: (template: string) => renderPrompt(workId, template),
    onSuccess: (prompt) => ask.mutate(prompt),
    onError: (cause) => say.failed(cause),
  })

  const busy = ask.isPending || runTemplate.isPending

  useEffect(() => {
    bottom.current?.scrollIntoView({ block: 'nearest' })
  }, [messages.length, busy])

  const send = (prompt: string) => {
    if (chatId === null || prompt.trim() === '' || busy) return
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
      </div>

      {profile.config.prompts.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {profile.config.prompts.map((prompt) => (
            <Button
              key={prompt.key}
              size="sm"
              title={prompt.description}
              disabled={busy}
              onClick={() => runTemplate.mutate(prompt.template)}
            >
              {prompt.label}
            </Button>
          ))}
        </div>
      )}

      {transcript.isPending && chatId !== null && <Skeleton className="h-20 w-full" />}

      {messages.length > 0 && (
        <ul className="flex max-h-96 flex-col gap-2 overflow-y-auto">
          {messages.map((message) => (
            <li
              key={message.id}
              className={cn(
                'rounded-xl px-3 py-2 text-sm',
                message.role === 'user' ? 'bg-soft' : 'border border-line',
              )}
            >
              <p className="whitespace-pre-wrap">{message.body}</p>
              {typeof message.meta.cost_usd === 'number' && (
                <p className="mt-1 text-xs text-dim">${message.meta.cost_usd.toFixed(3)}</p>
              )}
            </li>
          ))}
          <div ref={bottom} />
        </ul>
      )}

      {busy && <p className="text-sm text-dim">{t('assistant.thinking')}</p>}

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
          onChange={(event) => setDraft(event.target.value)}
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
          <Button type="submit" variant="primary" disabled={busy || draft.trim() === ''}>
            {t('assistant.send')}
          </Button>
        </div>
      </form>
    </section>
  )
}
