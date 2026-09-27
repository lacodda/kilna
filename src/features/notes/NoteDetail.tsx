import { useCallback, useState } from 'react'
import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ArrowUpRight, Link2, Pencil, Sprout, Trash2, X } from 'lucide-react'
import { deleteNote, updateNote } from '@/lib/api/notes'
import type { Note, NotePatch } from '@/lib/api/types'
import { getWork } from '@/lib/api/works'
import { toggleTask } from '@/lib/checklist'
import { keys } from '@/lib/query/keys'
import { say } from '@/lib/toast'
import { announceDeleted } from '@/lib/trash'
import { labelOf, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Markdown } from '@/components/Markdown'
import { Select } from '@/components/AppSelect'
import { SaveState, type SaveStatus } from '@/components/SaveState'
import { Textarea } from '@/components/ui/textarea'
import { PickWorkDialog } from '@/components/PickWorkDialog'
import { NoteTagAdder } from '@/features/notes/NoteTagAdder'
import { PromoteNoteDialog } from '@/features/notes/PromoteNoteDialog'
import { useNoteBody } from '@/features/notes/useNoteBody'

// What a note changes when it is edited: the lists that show it, the tags the
// rest of the app completes from, and the history line the edit writes.
const REFRESHED = [keys.notes, keys.tags, keys.journal] as const

interface Props {
  note: Note
  /** Every tag in use, for completing the next one. */
  tags: string[]
  /** Open straight into the editor — a note just made has nothing to read. */
  startEditing: boolean
  /** Narrow the list to a tag, from a click on one. */
  onTag: (tag: string) => void
  /** The note left: deleted, or promoted to a work. */
  onGone: () => void
}

/**
 * One note, open: its title and kind along the top with what can be done to
 * it, the body in the middle, the tags along the bottom.
 *
 * The body reads as rendered markdown — links resolve, checklists tick — and
 * turns into its text on "Edit". Reading is the default because a note is
 * opened far more often to be read, or to tick a box, than to be rewritten.
 */
export function NoteDetail({ note, tags, startEditing, onTag, onGone }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const client = useQueryClient()
  const { config } = useProfile()
  const kinds = config.note_kinds ?? []

  const [title, setTitle] = useState(note.title ?? '')
  const [editing, setEditing] = useState(startEditing)
  const [promoting, setPromoting] = useState(false)
  const [attaching, setAttaching] = useState(false)

  const settle = useCallback(() => {
    for (const key of REFRESHED) void client.invalidateQueries({ queryKey: key })
  }, [client])

  const patch = useMutation({
    mutationFn: (change: NotePatch) => updateNote(note.id, change),
    onSuccess: settle,
    onError: (cause) => say.failedTo(t('toast.noteSaveFailed'), cause),
  })

  const body = useNoteBody(note, settle, t('toast.noteSaveFailed'))

  const remove = useMutation({
    mutationFn: async () => {
      // Whatever is still pending goes in first, so the trash holds the note
      // as it was last seen and a restore brings back the last word.
      await body.flush()
      return deleteNote(note.id)
    },
    onSuccess: (deletionId) => {
      announceDeleted({
        client,
        deletionId,
        message: t('toast.noteDeleted'),
        refresh: REFRESHED,
      })
      onGone()
    },
    onError: (cause) => say.failedTo(t('toast.noteSaveFailed'), cause),
  })

  const work = useQuery({
    queryKey: keys.work(note.work_id ?? ''),
    queryFn: () => getWork(note.work_id!),
    enabled: note.work_id !== null,
  })

  const saveTitle = () => {
    const next = title.trim()
    if (next === (note.title ?? '')) return
    patch.mutate({ title: next === '' ? null : next })
  }

  // The kind the note carries stays pickable even when the profile no longer
  // names it: an old value shown rather than silently rewritten.
  const kindOptions = [
    ...kinds.map((one) => ({ value: one.key, label: labelOf(kinds, one.key) })),
    ...(kinds.some((one) => one.key === note.kind) ? [] : [{ value: note.kind, label: note.kind }]),
  ]

  const status: SaveStatus = body.status === 'saving' || patch.isPending ? 'saving' : body.status

  return (
    <section className="flex min-h-0 flex-col overflow-hidden rounded-xl border border-line bg-raise">
      <header className="flex shrink-0 flex-wrap items-center gap-2 border-b border-line px-3 py-2">
        <Input
          value={title}
          onChange={(event) => setTitle(event.target.value)}
          onBlur={saveTitle}
          onKeyDown={(event) => {
            if (event.key === 'Enter') event.currentTarget.blur()
          }}
          placeholder={t('notes.titlePlaceholder')}
          aria-label={t('notes.titleLabel')}
          className="min-w-40 flex-1 border-transparent bg-transparent px-1.5 text-[13px] font-semibold hover:border-line focus:border-line"
        />
        <SaveState status={status} />
        {kindOptions.length > 1 && (
          <Select
            value={note.kind}
            onChange={(next) => {
              if (next !== '' && next !== note.kind) patch.mutate({ kind: next })
            }}
            options={kindOptions}
            aria-label={t('notes.kind')}
            className="w-36"
          />
        )}
        <Button
          size="sm"
          onClick={() => {
            void body.flush().then(() => setPromoting(true))
          }}
          title={t('notes.promoteHint')}
        >
          <Sprout aria-hidden />
          {t('notes.promote')}
        </Button>
        <Button
          size="icon-sm"
          variant="danger"
          title={t('notes.delete')}
          aria-label={t('notes.delete')}
          disabled={remove.isPending}
          onClick={() => remove.mutate()}
        >
          <Trash2 aria-hidden />
        </Button>
      </header>

      {/* What the note is about, when it is about a work: one click to the
          card, one to let go. Attaching is how an idea written on its own
          joins the song it turned out to be for. */}
      <div className="flex shrink-0 items-center gap-1.5 border-b border-line px-3 py-1.5 text-xs text-dim">
        <Link2 aria-hidden className="size-3.5 text-faint" />
        {note.work_id === null ? (
          <button
            type="button"
            className="cursor-pointer text-faint hover:text-text"
            onClick={() => setAttaching(true)}
          >
            {t('notes.attach')}
          </button>
        ) : (
          <>
            <button
              type="button"
              className="flex min-w-0 cursor-pointer items-center gap-1 truncate hover:text-text"
              onClick={() => void navigate(`/works/${note.work_id ?? ''}`)}
            >
              <span className="truncate">{work.data?.title ?? '…'}</span>
              <ArrowUpRight aria-hidden className="size-3 shrink-0" />
            </button>
            <Button
              size="icon-sm"
              variant="icon"
              className="size-5"
              title={t('notes.detach')}
              aria-label={t('notes.detach')}
              onClick={() => patch.mutate({ work_id: null })}
            >
              <X aria-hidden />
            </Button>
          </>
        )}
        <Button
          size="xs"
          variant="ghost"
          className="ml-auto"
          aria-pressed={editing}
          onClick={() => {
            if (editing) void body.flush()
            setEditing(!editing)
          }}
        >
          <Pencil aria-hidden />
          {editing ? t('notes.done') : t('notes.edit')}
        </Button>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto">
        {editing ? (
          <Textarea
            autoFocus
            value={body.text}
            onChange={(event) => body.setText(event.target.value)}
            onKeyDown={(event) => {
              // Escape leaves the text the way it leaves any editor here.
              if (event.key === 'Escape') {
                void body.flush()
                setEditing(false)
              }
            }}
            placeholder={t('notes.bodyPlaceholder')}
            aria-label={t('notes.bodyLabel')}
            className="h-full min-h-full w-full resize-none rounded-none border-0 bg-transparent px-5 py-4 font-mono text-[13px] leading-relaxed focus-visible:ring-0"
          />
        ) : body.text.trim() === '' ? (
          <button
            type="button"
            className="w-full cursor-text px-5 py-4 text-left text-sm text-faint"
            onClick={() => setEditing(true)}
          >
            {t('notes.bodyPlaceholder')}
          </button>
        ) : (
          <div className="px-5 py-4" onDoubleClick={() => setEditing(true)}>
            <Markdown
              body={body.text}
              className="text-[13px]"
              onToggleTask={(index) => body.setText(toggleTask(body.text, index), true)}
            />
          </div>
        )}
      </div>

      <footer className="flex shrink-0 flex-wrap items-center gap-1.5 border-t border-line px-3 py-2">
        {note.tags.map((tag) => (
          <span
            key={tag}
            className="flex items-center gap-1 rounded-full border border-line py-0.5 pr-1 pl-2.5 text-[11.5px] text-dim"
          >
            <button
              type="button"
              className="cursor-pointer hover:text-text"
              title={t('notes.filterByTag', { tag })}
              onClick={() => onTag(tag)}
            >
              {tag}
            </button>
            <button
              type="button"
              className="flex size-4 cursor-pointer items-center justify-center rounded-full text-faint hover:bg-soft hover:text-text"
              aria-label={t('notes.removeTag', { tag })}
              onClick={() => patch.mutate({ tags: note.tags.filter((have) => have !== tag) })}
            >
              <X aria-hidden className="size-3" />
            </button>
          </span>
        ))}
        <NoteTagAdder
          have={note.tags}
          known={tags}
          onAdd={(tag) => {
            if (!note.tags.some((have) => have.toLowerCase() === tag.toLowerCase())) {
              patch.mutate({ tags: [...note.tags, tag] })
            }
          }}
        />
      </footer>

      <PromoteNoteDialog
        open={promoting}
        onOpenChange={setPromoting}
        note={{ ...note, body: body.text }}
        onPromoted={(workId) => void navigate(`/works/${workId}`)}
      />
      <PickWorkDialog
        open={attaching}
        onOpenChange={setAttaching}
        title={t('notes.attachTitle')}
        onPick={(picked) => patch.mutate({ work_id: picked.work_id })}
      />
    </section>
  )
}
