import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { GripVertical, Pin, PinOff, Plus, Sparkles, Undo2, X } from 'lucide-react'
import {
  createFocusNote,
  deleteFocusNote,
  dismissFinding,
  dismissedFindings,
  listFocusNotes,
  reorderFocusNotes,
  restoreFinding,
  startTask,
  updateFocusNote,
  type Dismissal,
  type FocusNote,
  type ScheduledRelease,
  type ScoredWork,
} from '@/lib/api'
import { dismissalKey, findings, visible, type Finding } from '@/lib/findings'
import { today } from '@/lib/month'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { cn } from '@/lib/utils'
import { labelOf, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { SectionLabel } from '@/components/ui/panel'
import { Skeleton } from '@/components/ui/Skeleton'

interface Props {
  works: readonly ScoredWork[]
  calendar: readonly ScheduledRelease[]
  /** Kinds already spoken for by a section above; saying them twice is noise. */
  skip?: readonly Finding['kind'][]
  onSelect: (workId: string, tab?: string) => void
}

/**
 * The focus board: what the workspace noticed, and what the person put there.
 *
 * Two halves that look alike and are not. A **finding** is derived on every
 * read and stored nowhere — it appears when its complaint becomes true and
 * leaves when it stops being true. A **note** is a line the person wrote, and
 * it stays until they rub it out.
 *
 * That difference is why only one half has a pin and an order: keeping a
 * finding at the top would promise to hold something that is about to vanish on
 * its own, and giving findings a hand-made order would put a second answer to
 * "what matters most" beside the one the code already derives. What the person
 * can say about a finding is that they have heard it — and hiding remembers the
 * *complaint*, so a changed one comes back.
 *
 * Read-only about the workspace, as since v0.28: the only thing a finding can
 * start is a profile action, which lands in a chat like every other task.
 */
export function FocusBoard({ works, calendar, skip = [], onSelect }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const client = useQueryClient()

  const dismissals = useQuery({ queryKey: keys.dismissals, queryFn: dismissedFindings })
  const notes = useQuery({ queryKey: keys.focusNotes, queryFn: listFocusNotes })

  const refresh = () => {
    void client.invalidateQueries({ queryKey: keys.focus })
  }

  const start = useMutation({
    mutationFn: ({ workId, action }: { workId: string; action: string }) =>
      startTask(workId, action),
    onSuccess: (started) => {
      void client.invalidateQueries({ queryKey: keys.activeTasks })
      void client.invalidateQueries({ queryKey: keys.allChats })
      say.info(t('assistant.taskStarted', { title: started.title }))
    },
    onError: say.failed,
  })

  const hide = useMutation({
    mutationFn: (finding: Finding) => dismissFinding(dismissalKey(finding)),
    onSuccess: refresh,
    onError: say.failed,
  })

  const unhide = useMutation({
    mutationFn: (row: Dismissal) =>
      restoreFinding({ kind: row.kind, work_id: row.work_id, complaint: row.complaint }),
    onSuccess: refresh,
    onError: say.failed,
  })

  const hidden = new Set(skip)
  const standing = visible(
    findings(works, calendar, profile.config, today()),
    dismissals.data ?? [],
  ).filter((finding) => !hidden.has(finding.kind))

  const board = notes.data ?? []
  const putAway = dismissals.data ?? []

  // The heading is earned by what stands under it, not by what was put away.
  // A "Worth doing" rule over an empty area with a lone `3 put away` beside it
  // reads as a section that failed to load — which is exactly how it looked
  // the first time every finding on a real workspace was dismissed.
  const anythingToShow = standing.length > 0 || board.length > 0

  // Until both queries land, whether there is anything to show is not yet
  // known — and the answer decides whether this section exists at all. Drawing
  // nothing meanwhile makes the board appear late and push the dashboard down;
  // drawing a full skeleton would promise a board that may legitimately be
  // empty (v0.34: the heading is earned by what stands under it). One row's
  // worth of space, and only while the answer is on its way.
  if (dismissals.isPending || notes.isPending) {
    return <Skeleton className="h-8 w-full" />
  }

  return (
    <section className="flex flex-col gap-3">
      {anythingToShow && (
        <SectionLabel>
          <Sparkles aria-hidden className="size-3.5" />
          {t('findings.title')}
          <span className="normal-case tracking-normal text-dim">{t('findings.readOnly')}</span>
        </SectionLabel>
      )}

      {standing.length > 0 && (
        <div className="flex flex-wrap gap-2">
          {standing.map((finding) => (
            <span
              key={`${finding.kind}:${finding.workId}`}
              className="flex items-center gap-2 rounded-xl border border-dashed border-line-2 px-3 py-1.5 text-[12.5px] text-dim"
            >
              <button
                type="button"
                onClick={() => onSelect(finding.workId, tabFor(finding.kind))}
                className="cursor-pointer text-left transition-colors hover:text-text"
              >
                {t(`findings.kind.${finding.kind}`, { title: finding.title })}
              </button>

              {finding.action !== undefined && (
                <Button
                  size="sm"
                  variant="soft"
                  disabled={start.isPending}
                  title={t('findings.askHint')}
                  onClick={() => {
                    start.mutate({ workId: finding.workId, action: finding.action as string })
                  }}
                >
                  {labelOf(profile.config.prompts, finding.action)}
                </Button>
              )}

              <button
                type="button"
                aria-label={t('findings.dismiss')}
                title={t('findings.dismissHint')}
                disabled={hide.isPending}
                onClick={() => hide.mutate(finding)}
                className="cursor-pointer text-faint transition-colors hover:text-text"
              >
                <X aria-hidden className="size-3.5" />
              </button>
            </span>
          ))}
        </div>
      )}

      <NoteList notes={board} onSelect={onSelect} />

      <div className="flex flex-wrap items-center gap-3">
        <AddNote />
        {putAway.length > 0 && <Hidden rows={putAway} onRestore={(row) => unhide.mutate(row)} />}
      </div>
    </section>
  )
}

/** The person's own lines, in the order they arranged them. */
function NoteList({
  notes,
  onSelect,
}: {
  notes: readonly FocusNote[]
  onSelect: (workId: string, tab?: string) => void
}) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const [dragging, setDragging] = useState<string | null>(null)
  const [over, setOver] = useState<string | null>(null)

  const refresh = () => {
    void client.invalidateQueries({ queryKey: keys.focusNotes })
  }

  const pin = useMutation({
    mutationFn: ({ id, pinned }: { id: string; pinned: boolean }) =>
      updateFocusNote(id, { pinned }),
    onSuccess: refresh,
    onError: say.failed,
  })

  const rub = useMutation({
    mutationFn: deleteFocusNote,
    onSuccess: refresh,
    onError: say.failed,
  })

  const move = useMutation({
    mutationFn: reorderFocusNotes,
    onSuccess: refresh,
    onError: say.failed,
  })

  if (notes.length === 0) return null

  /** Drop `dragging` where `target` sits, and send the whole arrangement. */
  const drop = (target: string) => {
    if (dragging === null || dragging === target) return

    const order = notes.map((note) => note.id).filter((id) => id !== dragging)
    order.splice(order.indexOf(target), 0, dragging)
    move.mutate(order)
  }

  return (
    <ul className="flex flex-col gap-1.5">
      {notes.map((note) => (
        <li
          key={note.id}
          onDragOver={(event) => {
            event.preventDefault()
            setOver(note.id)
          }}
          onDragLeave={() => setOver((current) => (current === note.id ? null : current))}
          onDrop={(event) => {
            event.preventDefault()
            drop(note.id)
            setOver(null)
          }}
          className={cn(
            'flex items-center gap-2 rounded-xl border border-line bg-raise px-3 py-2 text-[12.5px]',
            dragging === note.id && 'opacity-40',
            over === note.id && dragging !== note.id && 'border-accent',
          )}
        >
          {/* The handle, and only the handle, is draggable — the same rule the
              calendar follows, for the same reason: a whole draggable row puts
              every click in a race with a drag. */}
          <span
            draggable
            onDragStart={(event) => {
              event.dataTransfer.setData('text/plain', note.id)
              event.dataTransfer.effectAllowed = 'move'
              setDragging(note.id)
            }}
            onDragEnd={() => {
              setDragging(null)
              setOver(null)
            }}
            title={t('focus.dragHandle')}
            className="shrink-0 cursor-grab text-faint hover:text-dim"
          >
            <GripVertical aria-hidden className="size-3.5" />
          </span>

          {note.work_id === null ? (
            <span className="min-w-0 flex-1">{note.body}</span>
          ) : (
            <button
              type="button"
              onClick={() => onSelect(note.work_id as string)}
              className="min-w-0 flex-1 cursor-pointer text-left transition-colors hover:text-accent"
            >
              {note.body}
            </button>
          )}

          <button
            type="button"
            aria-label={note.pinned_at === null ? t('focus.pin') : t('focus.unpin')}
            title={note.pinned_at === null ? t('focus.pin') : t('focus.unpin')}
            onClick={() => pin.mutate({ id: note.id, pinned: note.pinned_at === null })}
            className={cn(
              'shrink-0 cursor-pointer transition-colors hover:text-text',
              note.pinned_at === null ? 'text-faint' : 'text-accent',
            )}
          >
            {note.pinned_at === null ? (
              <Pin aria-hidden className="size-3.5" />
            ) : (
              <PinOff aria-hidden className="size-3.5" />
            )}
          </button>

          <button
            type="button"
            aria-label={t('focus.remove')}
            title={t('focus.remove')}
            onClick={() => rub.mutate(note.id)}
            className="shrink-0 cursor-pointer text-faint transition-colors hover:text-bad"
          >
            <X aria-hidden className="size-3.5" />
          </button>
        </li>
      ))}
    </ul>
  )
}

/** One field, opened by a link rather than sitting on the screen unused. */
function AddNote() {
  const { t } = useTranslation()
  const client = useQueryClient()
  const [body, setBody] = useState('')
  const [open, setOpen] = useState(false)

  const add = useMutation({
    mutationFn: (line: string) => createFocusNote({ body: line }),
    onSuccess: () => {
      setBody('')
      setOpen(false)
      void client.invalidateQueries({ queryKey: keys.focusNotes })
    },
    onError: say.failed,
  })

  const submit = () => {
    const line = body.trim()
    if (line !== '') add.mutate(line)
  }

  if (!open) {
    return (
      <button
        type="button"
        onClick={() => setOpen(true)}
        className="flex cursor-pointer items-center gap-1.5 text-[12.5px] text-faint transition-colors hover:text-text"
      >
        <Plus aria-hidden className="size-3.5" />
        {t('focus.add')}
      </button>
    )
  }

  return (
    <div className="flex flex-1 items-center gap-2">
      <Input
        autoFocus
        value={body}
        placeholder={t('focus.addPlaceholder')}
        onChange={(event) => setBody(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === 'Enter') submit()
          // Escape gives up on the line rather than leaving an open field
          // nobody asked for.
          if (event.key === 'Escape') {
            setBody('')
            setOpen(false)
          }
        }}
      />
      <Button size="sm" variant="soft" disabled={add.isPending} onClick={submit}>
        {t('focus.save')}
      </Button>
    </div>
  )
}

/**
 * What has been put away, and the way back.
 *
 * Folded behind a count rather than listed: hiding is meant to quieten the
 * board, and a permanent list of everything dismissed would undo that. It still
 * has to be reachable — a complaint hidden by mistake is otherwise gone for as
 * long as it keeps saying the same thing.
 */
function Hidden({
  rows,
  onRestore,
}: {
  rows: readonly Dismissal[]
  onRestore: (row: Dismissal) => void
}) {
  const { t } = useTranslation()
  const [open, setOpen] = useState(false)

  return (
    <div className="flex flex-wrap items-center gap-2">
      <button
        type="button"
        onClick={() => setOpen((current) => !current)}
        aria-expanded={open}
        className="cursor-pointer text-[12.5px] text-faint transition-colors hover:text-text"
      >
        {t('focus.hiddenCount', { count: rows.length })}
      </button>

      {open &&
        rows.map((row) => (
          <button
            key={`${row.kind}:${row.work_id}:${row.complaint}`}
            type="button"
            onClick={() => onRestore(row)}
            title={t('focus.restoreHint')}
            className="flex cursor-pointer items-center gap-1.5 rounded-full border border-line px-2.5 py-0.5 text-[11.5px] text-faint transition-colors hover:border-line-2 hover:text-text"
          >
            <Undo2 aria-hidden className="size-3" />
            {t(`findings.kindShort.${row.kind}`)}
          </button>
        ))}
    </div>
  )
}

/** Where a complaint is answered — the tab that closes it, when there is one. */
function tabFor(kind: Finding['kind']): string | undefined {
  switch (kind) {
    case 'unscored':
    case 'stale-score':
      return 'score'
    case 'ready-unscheduled':
    case 'weak-scheduled':
      return 'releases'
    case 'stale-draft':
      return undefined
  }
}
