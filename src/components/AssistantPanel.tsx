import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  askAssistant,
  assistantStatus,
  createChat,
  getTranscript,
  listChats,
  renderPrompt,
  type Availability,
  type Message,
} from '@/lib/api'
import { useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/Button'
import { Textarea } from '@/components/ui/Input'
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

  const [status, setStatus] = useState<Availability | null>(null)
  const [chatId, setChatId] = useState<string | null>(null)
  const [messages, setMessages] = useState<Message[]>([])
  const [draft, setDraft] = useState('')
  const [pending, setPending] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const bottom = useRef<HTMLDivElement>(null)

  useEffect(() => {
    assistantStatus()
      .then(setStatus)
      .catch((cause: unknown) => setError(String(cause)))
  }, [])

  // One chat per work, reused across sessions.
  useEffect(() => {
    listChats(workId)
      .then((chats) => chats[0] ?? createChat({ work_id: workId }))
      .then((chat) => setChatId(chat.id))
      .catch((cause: unknown) => setError(String(cause)))
  }, [workId])

  useEffect(() => {
    if (chatId === null) return

    getTranscript(chatId)
      .then((transcript) => setMessages(transcript?.messages ?? []))
      .catch((cause: unknown) => setError(String(cause)))
  }, [chatId])

  useEffect(() => {
    bottom.current?.scrollIntoView({ block: 'nearest' })
  }, [messages, pending])

  const send = (prompt: string) => {
    if (chatId === null || prompt.trim() === '' || pending) return

    // Show the question immediately; the backend has already stored it.
    setMessages((current) => [
      ...current,
      {
        id: `pending-${String(current.length)}`,
        chat_id: chatId,
        role: 'user',
        body: prompt,
        meta: {},
        created_at: new Date().toISOString(),
      },
    ])
    setDraft('')
    setPending(true)
    setError(null)

    askAssistant(chatId, prompt)
      .then(() => getTranscript(chatId))
      .then((transcript) => setMessages(transcript?.messages ?? []))
      .catch((cause: unknown) => setError(String(cause)))
      .finally(() => setPending(false))
  }

  const runTemplate = (template: string) => {
    renderPrompt(workId, template)
      .then(send)
      .catch((cause: unknown) => setError(String(cause)))
  }

  if (status !== null && !status.available) {
    return (
      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-semibold">{t('assistant.title')}</h3>
        <p className="rounded-md border border-dashed border-neutral-300 p-4 text-sm text-neutral-500 dark:border-neutral-700">
          {status.reason ?? t('assistant.unavailable')}
        </p>
      </section>
    )
  }

  return (
    <section className="flex flex-col gap-3">
      <div className="flex items-center gap-3">
        <h3 className="text-sm font-semibold">{t('assistant.title')}</h3>
        {status?.version != null && (
          <span className="text-xs text-neutral-500">{status.version}</span>
        )}
      </div>

      {profile.config.prompts.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {profile.config.prompts.map((prompt) => (
            <Button
              key={prompt.key}
              size="sm"
              title={prompt.description}
              disabled={pending}
              onClick={() => runTemplate(prompt.template)}
            >
              {prompt.label}
            </Button>
          ))}
        </div>
      )}

      {error !== null && (
        <p role="alert" className="text-sm text-red-600">
          {error}
        </p>
      )}

      {messages.length > 0 && (
        <ul className="flex max-h-96 flex-col gap-2 overflow-y-auto">
          {messages.map((message) => (
            <li
              key={message.id}
              className={cn(
                'rounded-md px-3 py-2 text-sm',
                message.role === 'user'
                  ? 'bg-neutral-100 dark:bg-neutral-800'
                  : 'border border-neutral-200 dark:border-neutral-700',
              )}
            >
              <p className="whitespace-pre-wrap">{message.body}</p>
              {typeof message.meta.cost_usd === 'number' && (
                <p className="mt-1 text-xs text-neutral-500">
                  ${message.meta.cost_usd.toFixed(3)}
                </p>
              )}
            </li>
          ))}
          <div ref={bottom} />
        </ul>
      )}

      {pending && <p className="text-sm text-neutral-500">{t('assistant.thinking')}</p>}

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
          <Button type="submit" variant="primary" disabled={pending || draft.trim() === ''}>
            {t('assistant.send')}
          </Button>
        </div>
      </form>
    </section>
  )
}
