import { useEffect, useRef, useState } from 'react'
import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { listen } from '@tauri-apps/api/event'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import {
  applyPendingProposals,
  cancelRun,
  createChat,
  getTranscript,
  listRuns,
  renderPrompt,
  startRun,
  type Run,
  type RunEmission,
} from '@/lib/api'
import { actionsOfScope } from '@/lib/actions'
import { conversation, pending, type Exchange } from '@/lib/chat'
import { reading } from '@/lib/palette'
import { formatDuration, withEvent } from '@/lib/runs'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import { Markdown } from '@/components/ui/Markdown'
import { Skeleton } from '@/components/ui/Skeleton'
import { InsertVersionDialog } from '@/components/assistant/InsertVersionDialog'
import { KeepAsNoteDialog } from '@/components/assistant/KeepAsNoteDialog'
import { ProposedNote } from '@/components/assistant/ProposedNote'
import { ProposedScenes } from '@/components/assistant/ProposedScenes'
import { ProposedScore } from '@/components/assistant/ProposedScore'
import { ProposedVersion } from '@/components/assistant/ProposedVersion'
import { ProposedWork } from '@/components/assistant/ProposedWork'

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
  const navigate = useNavigate()
  const profile = useProfile()
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
  // Which entry of the slash palette the arrow keys are on. Reset whenever the
  // query changes, so the highlight never points past a shortened list.
  const [highlighted, setHighlighted] = useState(0)
  const bottom = useRef<HTMLDivElement>(null)
  const composer = useRef<HTMLTextAreaElement>(null)

  const transcript = useQuery({
    queryKey: keys.transcript(chatId ?? ''),
    queryFn: () => getTranscript(chatId!),
    enabled: chatId !== null,
    // A proposal from outside the window — `kilna --mcp` — lands in this
    // chat without an event; while the chat is open it appears within a
    // few seconds rather than on the next click.
    refetchInterval: 10_000,
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

  // The palette is open when the draft is nothing but a slash command. Derived
  // rather than kept in state: the draft is the only truth, and a second copy
  // would be one more thing to get out of step with it.
  // A work chat offers the work's actions: a scene's, a style's or a
  // comment's template filled into it would ask about something it is not.
  const workActions = actionsOfScope(profile.config.prompts, 'work')
  const palette = workId === undefined ? null : reading(draft, workActions)
  const chosen = palette?.matches[Math.min(highlighted, palette.matches.length - 1)] ?? null

  const items = conversation(transcript.data?.messages ?? [], runs.data ?? [])
  const working = items.some((item) => item.run?.working === true)
  const sending = ask.isPending || runTemplate.isPending
  const waiting = pending(items)

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
  }, [items.length, working])

  // Choosing from the palette does what its button does: render the template
  // into the composer, to be read before it is sent. The palette is a faster
  // way to reach the same action, not a second meaning for it.
  const pick = (action: { template: string }) => {
    setHighlighted(0)
    runTemplate.mutate(action.template)
  }

  const send = (prompt: string) => {
    if (prompt.trim() === '' || sending) return
    ask.mutate(prompt)
  }

  const copy = (body: string) => {
    navigator.clipboard.writeText(body).then(
      () => {
        say.ok(t('assistant.copied'))
      },
      (cause: unknown) => {
        say.failed(cause)
      },
    )
  }

  const loading = chatId !== null && (transcript.isPending || runs.isPending)

  return (
    <div className="flex flex-col gap-3">
      {working && <span className="text-xs text-dim">{t('assistant.thinking')}</span>}

      {workId !== undefined && workActions.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {workActions.map((prompt) => (
            <Button
              key={prompt.key}
              size="sm"
              title={sayLabel(prompt.description)}
              disabled={sending}
              onClick={() => {
                runTemplate.mutate(prompt.template)
              }}
            >
              {sayLabel(prompt.label)}
            </Button>
          ))}
        </div>
      )}

      {loading && <Skeleton className="h-20 w-full" />}

      {chatId !== null && waiting.length > 1 && (
        <div className="flex items-center gap-2 rounded-xl border border-line bg-soft px-3 py-2 text-sm">
          <span className="text-dim">{t('assistant.pendingCount', { count: waiting.length })}</span>
          <Button
            className="ml-auto"
            size="sm"
            variant="primary"
            disabled={applyAll.isPending}
            onClick={() => {
              applyAll.mutate()
            }}
          >
            {t('assistant.applyAll')}
          </Button>
        </div>
      )}

      {items.length > 0 && (
        <ul className="flex max-h-96 flex-col gap-3 overflow-y-auto">
          {items.map((item) => (
            <ExchangeItem
              key={item.key}
              item={item}
              workId={workId}
              onCopy={copy}
              onInsert={
                workId === undefined
                  ? undefined
                  : (body, role, label, messageId) => setInserting({ body, role, label, messageId })
              }
              onKeepAsNote={(body) => setKeeping(body)}
              onOpenComments={() => void navigate('/comments')}
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
        className="relative flex flex-col gap-2"
        onSubmit={(event) => {
          event.preventDefault()
          if (palette !== null) {
            if (chosen !== null) pick(chosen)
            return
          }
          send(draft)
        }}
      >
        {palette !== null && (
          <ul
            role="listbox"
            aria-label={t('assistant.paletteLabel')}
            className="absolute bottom-full z-10 mb-1 max-h-64 w-full overflow-y-auto rounded-xl border border-line bg-raise p-1 shadow-raise"
          >
            {palette.matches.length === 0 && (
              <li className="px-2.5 py-2 text-sm text-dim">{t('assistant.paletteEmpty')}</li>
            )}
            {palette.matches.map((action, index) => (
              <li key={action.key}>
                <button
                  type="button"
                  role="option"
                  aria-selected={action.key === chosen?.key}
                  // Pointer down, not click: the composer keeps focus, so the
                  // draft the choice replaces is still the one on screen.
                  onMouseDown={(event) => {
                    event.preventDefault()
                    pick(action)
                  }}
                  onMouseEnter={() => {
                    setHighlighted(index)
                  }}
                  className={cn(
                    'flex w-full cursor-pointer flex-col gap-0.5 rounded-md px-2.5 py-1.5 text-left',
                    action.key === chosen?.key ? 'bg-soft text-text' : 'text-dim',
                  )}
                >
                  <span className="text-sm">{sayLabel(action.label)}</span>
                  {action.description !== undefined && (
                    <span className="truncate text-xs text-faint">
                      {sayLabel(action.description)}
                    </span>
                  )}
                </button>
              </li>
            ))}
          </ul>
        )}

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
            // While the palette is open the arrows and Enter belong to it.
            if (palette !== null) {
              if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
                event.preventDefault()
                const count = palette.matches.length
                if (count > 0) {
                  const step = event.key === 'ArrowDown' ? 1 : count - 1
                  setHighlighted((current) => (Math.min(current, count - 1) + step) % count)
                }
                return
              }
              if (event.key === 'Escape') {
                event.preventDefault()
                setDraft('')
                return
              }
              if (event.key === 'Enter' && !event.shiftKey) {
                event.preventDefault()
                if (chosen !== null) pick(chosen)
                return
              }
            }

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
    </div>
  )
}

function ExchangeItem({
  item,
  workId,
  onCopy,
  onInsert,
  onKeepAsNote,
  onOpenComments,
  onStop,
  stopping,
}: {
  item: Exchange
  /** Absent when the chat is about nothing — there is nothing to score. */
  workId?: string
  onCopy: (body: string) => void
  /** Absent when the chat is about nothing — there is no work to version. */
  onInsert?: (body: string, role?: string, label?: string, messageId?: string) => void
  /** Keep the answer as a note. Always offered: a note needs no work. */
  onKeepAsNote: (body: string) => void
  /** Go where a comment or a reply is kept. */
  onOpenComments: () => void
  onStop: (runId: string) => void
  stopping: boolean
}) {
  const { t } = useTranslation()
  const run = item.run

  // The settled message is canonical; a run's own body stands in while it is
  // still growing, or when the run ended before an answer was stored.
  const body = item.answer?.body ?? run?.body ?? ''
  const cost = item.answer?.cost ?? run?.cost ?? null
  // What the run cost in time as well as in money: the two facts the CLI
  // reports about a finished turn, side by side under the answer.
  const took = formatDuration(item.answer?.durationMs ?? run?.durationMs ?? null)
  const proposal = item.answer?.proposal ?? null
  const applied = item.answer?.applied ?? null
  const settled = item.run?.working !== true
  // A proposed version, package or storyboard has its own buttons below; the
  // toolbar's *insert* on it would keep a rendering of a package as a lyric.
  // A comment or a reply is kept on the comments screen, where its fields
  // can be corrected first; neither is a version or a note of anything.
  const aboutComment = proposal?.kind === 'comment' || proposal?.kind === 'reply'
  const insertable =
    onInsert !== undefined &&
    settled &&
    !aboutComment &&
    proposal?.kind !== 'version' &&
    proposal?.kind !== 'work' &&
    proposal?.kind !== 'scenes'
  // Keeping needs no work, so it is offered wherever inserting would be minus
  // that condition — and not on an answer that already proposed a note, which
  // has its own button and would otherwise be kept twice.
  const keepable =
    settled &&
    !aboutComment &&
    proposal?.kind !== 'note' &&
    proposal?.kind !== 'version' &&
    proposal?.kind !== 'work' &&
    proposal?.kind !== 'scenes'

  return (
    <li className="flex flex-col gap-1.5">
      {item.prompt !== null && (
        <p className="selectable rounded-xl bg-soft px-3 py-2 text-sm whitespace-pre-wrap">{item.prompt}</p>
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
          {/* Who said it, when it was not the assistant in this panel: an
              agent outside the window, named by its client, with what it said
              about its proposal. */}
          {item.answer?.source != null && (
            <p className="mb-1.5 text-xs text-dim">
              {t('assistant.proposedBy', { client: item.answer.source })}
              {item.answer.note != null && item.answer.note !== '' && (
                <>
                  {' — '}
                  {item.answer.note}
                </>
              )}
            </p>
          )}
          <Markdown body={body} copyLabel={t('assistant.copy')} />
          <div className="mt-1.5 flex items-center gap-1.5">
            {cost != null && <span className="text-xs text-dim">${cost.toFixed(3)}</span>}
            {took !== null && <span className="text-xs text-faint">{took}</span>}
            {/* An answer still growing is not worth keeping yet. */}
            {insertable && (
              <Button
                size="xs"
                variant="ghost"
                className="ml-auto"
                onClick={() => {
                  onInsert(body)
                }}
              >
                {t('assistant.insert')}
              </Button>
            )}
            {/* Beside inserting, and on the same condition: an answer still
                growing is not worth keeping yet, and a proposal with its own
                buttons below is kept by those. */}
            {keepable && (
              <Button
                size="xs"
                variant="ghost"
                className={insertable ? undefined : 'ml-auto'}
                onClick={() => {
                  onKeepAsNote(body)
                }}
              >
                {t('assistant.keepAsNote')}
              </Button>
            )}
            <Button
              size="xs"
              variant="ghost"
              className={insertable || keepable ? undefined : 'ml-auto'}
              onClick={() => {
                onCopy(body)
              }}
            >
              {t('assistant.copy')}
            </Button>
          </div>
        </div>
      )}

      {/* The action asked for something and the answer's block could not be
          read as it: said, so a button that never appears is never silent. */}
      {item.answer?.refused != null && settled && (
        <p role="alert" className="mx-3 text-xs text-warn">
          {t('assistant.proposalRefused', { why: item.answer.refused })}
        </p>
      )}

      {/* What the answer proposed, with the button that applies it. Below the
          answer rather than beside the copy buttons: it is a decision, not a
          convenience, and it needs room to show the numbers first. */}
      {workId !== undefined && item.answer !== null && proposal?.kind === 'score' && settled && (
        <ProposedScore
          workId={workId}
          messageId={item.answer.id}
          proposal={proposal}
          applied={applied}
        />
      )}
      {aboutComment && settled && (
        <p className="mx-3 flex items-center gap-2 text-xs text-dim">
          {applied !== null ? t('assistant.commentKept') : t('assistant.commentToKeep')}
          {applied === null && (
            <Button size="xs" variant="ghost" onClick={onOpenComments}>
              {t('assistant.openComments')}
            </Button>
          )}
        </p>
      )}
      {item.answer !== null && proposal?.kind === 'note' && settled && (
        <ProposedNote messageId={item.answer.id} proposal={proposal} applied={applied} />
      )}
      {workId !== undefined &&
        item.answer !== null &&
        onInsert !== undefined &&
        proposal?.kind === 'version' &&
        settled && (
          <ProposedVersion
            workId={workId}
            messageId={item.answer.id}
            proposal={proposal}
            applied={applied}
            onChoose={() => {
              onInsert(body, proposal.role, proposal.label, item.answer?.id)
            }}
          />
        )}
      {item.answer !== null && proposal?.kind === 'work' && settled && (
        <ProposedWork
          workId={workId}
          messageId={item.answer.id}
          proposal={proposal}
          applied={applied}
        />
      )}
      {workId !== undefined && item.answer !== null && proposal?.kind === 'scenes' && settled && (
        <ProposedScenes
          workId={workId}
          messageId={item.answer.id}
          proposal={proposal}
          applied={applied}
        />
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
