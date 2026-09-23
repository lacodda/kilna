import { useState } from 'react'
import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ArrowUpRight, X } from 'lucide-react'
import { createNote, deleteNote, listNotes, updateNote } from '@/lib/api'
import { toggleTask } from '@/lib/checklist'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { announceDeleted } from '@/lib/trash'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Markdown } from '@/components/ui/Markdown'
import { Textarea } from '@/components/ui/textarea'
import { Skeleton } from '@/components/ui/Skeleton'

// What a note changes when it appears or goes. A new tag on a note changes the
// tag list the rest of the app reads, so both are refreshed either way.
const REFRESHED = [keys.notes, keys.tags] as const

interface Props {
  workId: string
}

export function NotePanel({ workId }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const client = useQueryClient()
  const [body, setBody] = useState('')
  const [tags, setTags] = useState('')

  const notes = useQuery({
    queryKey: [...keys.notes, workId],
    queryFn: () => listNotes({ work_id: workId }),
  })

  const settle = () => {
    for (const key of REFRESHED) void client.invalidateQueries({ queryKey: key })
  }

  const add = useMutation({
    mutationFn: () =>
      createNote({
        body: body.trim(),
        work_id: workId,
        // Comma-separated in the field, an array in the store.
        tags: tags
          .split(',')
          .map((tag) => tag.trim())
          .filter((tag) => tag !== ''),
      }),
    onSuccess: () => {
      setBody('')
      setTags('')
      settle()
    },
    onError: (cause) => say.failedTo(t('toast.noteSaveFailed'), cause),
  })

  // A box ticked here is the same edit as one ticked on the notes screen: the
  // body is rewritten and the box follows it.
  const tick = useMutation({
    mutationFn: ({ id, next }: { id: string; next: string }) => updateNote(id, { body: next }),
    onSuccess: settle,
    onError: (cause) => say.failedTo(t('toast.noteSaveFailed'), cause),
  })

  const remove = useMutation({
    mutationFn: deleteNote,
    onSuccess: (deletionId) =>
      announceDeleted({
        client,
        deletionId,
        message: t('toast.noteDeleted'),
        refresh: REFRESHED,
      }),
    onError: (cause) => say.failedTo(t('toast.noteSaveFailed'), cause),
  })

  return (
    <section className="flex flex-col gap-3">
      {notes.isPending && <Skeleton className="h-16 w-full" />}

      {notes.isError && (
        <p role="alert" className="text-sm text-bad">
          {t('toast.loadFailed')}
        </p>
      )}

      {notes.data != null && notes.data.length > 0 && (
        <ul className="flex flex-col gap-2">
          {notes.data.map((note) => (
            <li
              key={note.id}
              className="flex items-start gap-2 rounded-xl border border-line p-2.5"
            >
              <div className="flex-1">
                {/* Rendered, not shown raw: a note is where a table of images
                    or a list of phrases lands, and pipes and asterisks are not
                    what its author wrote it to be read as. Line breaks inside a
                    paragraph are kept, as everywhere markdown is rendered here. */}
                <Markdown
                  body={note.body}
                  className="text-sm"
                  onToggleTask={(index) =>
                    tick.mutate({ id: note.id, next: toggleTask(note.body, index) })
                  }
                />
                {note.tags.length > 0 && (
                  <p className="mt-1 flex flex-wrap gap-1">
                    {note.tags.map((tag) => (
                      <span key={tag} className="rounded bg-soft px-1.5 py-0.5 text-xs text-dim">
                        {tag}
                      </span>
                    ))}
                  </p>
                )}
              </div>
              {/* Edited on the notes screen, where a note has room: the
                  card is the view from one work, not a second editor. */}
              <Button
                variant="icon"
                size="icon-sm"
                title={t('notes.openInNotes')}
                aria-label={t('notes.openInNotes')}
                onClick={() => void navigate(`/notes/${note.id}`)}
              >
                <ArrowUpRight aria-hidden className="size-3.5" />
              </Button>
              <Button
                variant="danger"
                size="icon-sm"
                title={t('notes.delete')}
                onClick={() => remove.mutate(note.id)}
              >
                <X aria-hidden className="size-3.5" />
              </Button>
            </li>
          ))}
        </ul>
      )}

      <form
        className="flex flex-col gap-2"
        onSubmit={(event) => {
          event.preventDefault()
          if (body.trim() !== '') add.mutate()
        }}
      >
        <Textarea
          rows={2}
          value={body}
          onChange={(event) => setBody(event.target.value)}
          placeholder={t('notes.placeholder')}
          aria-label={t('notes.placeholder')}
        />
        <div className="flex gap-2">
          <Input
            value={tags}
            onChange={(event) => setTags(event.target.value)}
            placeholder={t('notes.tagsPlaceholder')}
            aria-label={t('notes.tagsPlaceholder')}
          />
          <Button type="submit" variant="primary" disabled={body.trim() === '' || add.isPending}>
            {t('notes.add')}
          </Button>
        </div>
      </form>
    </section>
  )
}
