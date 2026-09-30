import { useMemo, useState } from 'react'
import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { ArrowUpRight, Ellipsis, Send } from 'lucide-react'
import { createNote, deleteNote, updateNote } from '@/lib/api/notes'
import type { Note, NoteFilter, NotePatch, NoteState } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { announceDeleted } from '@/lib/trash'
import { say } from '@/lib/toast'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { Input } from '@/components/ui/input'
import { Menu, MenuItem, MenuPopup, MenuSeparator, MenuTrigger } from '@/components/ui/menu'
import { SkeletonList } from '@/components/ui/skeleton'
import { Pane } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { PickWorkDialog } from '@/components/PickWorkDialog'
import { PromoteNoteDialog } from '@/features/notes/PromoteNoteDialog'

/** How many rows are drawn at first, and how many more each "show more". */
const PAGE = 200

interface Props {
  /** The kind of line the bank keeps: a phrase. */
  kind: string
  /** The narrowing the screen's head already chose: state, tag, words. */
  filter: NoteFilter
  /** The tag the screen narrows to, which a new line is given. */
  tag: string
  /** Whether anything narrows the list, for the empty state's words. */
  filtered: boolean
}

/**
 * A bank of lines, one per row (ADR 0045).
 *
 * The predecessor kept 1,131 phrases as cards, a wall nobody read past the
 * third day. Here each is a line under the last, with the work it went into
 * and what can be done to it at the end of the row: into a work in one
 * gesture - it stays, marked used - or set aside, dropped, made a work of.
 * A new line is typed into the box at the top and kept with Enter.
 */
export function LineBank({ kind, filter, tag, filtered }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const lines = useQuery(queries.notesMatching(filter))
  const catalogue = useQuery(queries.catalogue())
  const [draft, setDraft] = useState('')
  const [shown, setShown] = useState(PAGE)
  const [sending, setSending] = useState<Note | null>(null)
  const [promoting, setPromoting] = useState<Note | null>(null)

  const titles = useMemo(
    () => new Map((catalogue.data ?? []).map((row) => [row.work_id, row.title])),
    [catalogue.data],
  )

  const add = useAppMutation({
    mutationFn: (body: string) => createNote({ body, kind, tags: tag === '' ? [] : [tag] }),
    failure: 'toast.noteSaveFailed',
    refresh: refresh.note,
    onSuccess: () => setDraft(''),
  })
  const patch = useAppMutation({
    mutationFn: ({ id, change }: { id: string; change: NotePatch }) => updateNote(id, change),
    failure: 'toast.noteSaveFailed',
    refresh: refresh.note,
  })
  const remove = useAppMutation({
    mutationFn: (id: string) => deleteNote(id),
    failure: 'toast.noteSaveFailed',
    onSuccess: (deletionId) =>
      announceDeleted({
        client,
        deletionId,
        message: t('toast.noteDeleted'),
        refresh: refresh.note,
      }),
  })

  const rows = lines.data ?? []

  return (
    <Pane
      label={t('nav.notes')}
      bodyClassName="p-1.5"
      head={
        <Input
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={(event) => {
            const body = draft.trim()
            if (event.key === 'Enter' && body !== '' && !add.isPending) add.mutate(body)
          }}
          placeholder={t('notes.linePlaceholder')}
          aria-label={t('notes.linePlaceholder')}
          className="flex-1"
        />
      }
      foot={
        rows.length > shown ? (
          <>
            <span className="text-xs text-faint">
              {t('notes.linesShown', { shown, total: rows.length })}
            </span>
            <Button size="xs" variant="ghost" onClick={() => setShown(shown + PAGE)}>
              {t('notes.showMore')}
            </Button>
          </>
        ) : undefined
      }
    >
      <Loaded
        query={lines}
        skeleton={<SkeletonList rows={8} />}
        isEmpty={(data) => data.length === 0}
        emptyState={
          <EmptyState
            plain
            variant={filtered ? 'filtered' : 'empty'}
            title={filtered ? t('notes.noMatches') : t('notes.none')}
            className="p-2"
          />
        }
        plain
      >
        {() => (
          <ul className="flex flex-col">
            {rows.slice(0, shown).map((line) => (
              <Line
                key={line.id}
                line={line}
                work={line.work_id === null ? null : (titles.get(line.work_id) ?? '…')}
                onPatch={(change) => patch.mutate({ id: line.id, change })}
                onSend={() => setSending(line)}
                onPromote={() => setPromoting(line)}
                onDelete={() => remove.mutate(line.id)}
              />
            ))}
          </ul>
        )}
      </Loaded>

      <PickWorkDialog
        open={sending !== null}
        onOpenChange={(open) => {
          if (!open) setSending(null)
        }}
        title={t('notes.toWorkTitle')}
        onPick={(picked) => {
          if (sending === null) return
          // One gesture: tied to the work and marked used, together.
          patch.mutate(
            { id: sending.id, change: { work_id: picked.work_id, state: 'used' } },
            { onSuccess: () => say.ok(t('notes.spent', { title: picked.title })) },
          )
        }}
      />
      {promoting !== null && (
        <PromoteNoteDialog
          open
          onOpenChange={(open) => {
            if (!open) setPromoting(null)
          }}
          note={promoting}
          onPromoted={() => setPromoting(null)}
        />
      )}
    </Pane>
  )
}

interface LineProps {
  line: Note
  /** The title of the work it went into, when it went into one. */
  work: string | null
  onPatch: (change: NotePatch) => void
  onSend: () => void
  onPromote: () => void
  onDelete: () => void
}

/** One line of the bank: its words, its tags, where it went, what to do. */
function Line({ line, work, onPatch, onSend, onPromote, onDelete }: LineProps) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const [editing, setEditing] = useState(false)
  const [text, setText] = useState(line.body)

  const save = () => {
    setEditing(false)
    const next = text.trim()
    if (next !== '' && next !== line.body) onPatch({ body: next })
    else setText(line.body)
  }
  // The ways out of the state it is in: never the one it is already in.
  const moves: { state: NoteState; label: string }[] = [
    { state: 'parked' as const, label: t('notes.park') },
    { state: 'dropped' as const, label: t('notes.drop') },
    { state: 'fresh' as const, label: t('notes.refresh') },
  ].filter((move) => move.state !== line.state)

  return (
    <li
      className={cn(
        'group flex min-w-0 items-center gap-2 rounded-md px-2 py-1 text-sm hover:bg-soft',
        line.state !== 'fresh' && 'text-dim',
      )}
    >
      {editing ? (
        <Input
          autoFocus
          value={text}
          onChange={(event) => setText(event.target.value)}
          onBlur={save}
          onKeyDown={(event) => {
            if (event.key === 'Enter') event.currentTarget.blur()
            if (event.key === 'Escape') {
              setText(line.body)
              setEditing(false)
            }
          }}
          aria-label={t('notes.lineEdit')}
          className="min-w-0 flex-1"
        />
      ) : (
        // The words are to be read and copied: a double-click - or the
        // menu - makes them a field.
        <span
          className="selectable min-w-0 flex-1 truncate"
          title={line.body}
          onDoubleClick={() => setEditing(true)}
        >
          {line.body}
        </span>
      )}
      {line.tags.length > 0 && (
        <span className="max-w-48 shrink-0 truncate text-xs text-faint">
          {line.tags.join(', ')}
        </span>
      )}
      {line.state !== 'fresh' && (
        <span className="shrink-0 text-xs text-faint">{t(`notes.state.${line.state}`)}</span>
      )}
      {work !== null && line.work_id !== null && (
        <Button
          variant="link"
          className="max-w-48 min-w-0 shrink-0 text-xs"
          title={t('notes.usedIn', { title: work })}
          onClick={() => void navigate(`/works/${line.work_id ?? ''}`)}
        >
          <span className="truncate">{work}</span>
          <ArrowUpRight aria-hidden />
        </Button>
      )}
      <Button
        size="icon-xs"
        variant="icon"
        title={t('notes.toWork')}
        aria-label={t('notes.toWork')}
        onClick={onSend}
      >
        <Send aria-hidden />
      </Button>
      <Menu>
        <MenuTrigger
          render={
            <Button
              size="icon-xs"
              variant="icon"
              title={t('notes.lineActions')}
              aria-label={t('notes.lineActions')}
            />
          }
        >
          <Ellipsis aria-hidden />
        </MenuTrigger>
        <MenuPopup align="end">
          <MenuItem onClick={() => setEditing(true)}>{t('notes.lineEdit')}</MenuItem>
          <MenuItem onClick={onSend}>{t('notes.toWork')}</MenuItem>
          <MenuItem onClick={onPromote}>{t('notes.promote')}</MenuItem>
          <MenuSeparator />
          {moves.map((move) => (
            <MenuItem key={move.state} onClick={() => onPatch({ state: move.state })}>
              {move.label}
            </MenuItem>
          ))}
          <MenuSeparator />
          <MenuItem tone="danger" onClick={onDelete}>
            {t('notes.delete')}
          </MenuItem>
        </MenuPopup>
      </Menu>
    </li>
  )
}
