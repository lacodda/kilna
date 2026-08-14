import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { X } from 'lucide-react'
import { createNote, deleteNote, listNotes } from '@/lib/api'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/Button'
import { Input, Textarea } from '@/components/ui/Input'
import { Skeleton } from '@/components/ui/Skeleton'

interface Props {
  workId: string
}

export function NotePanel({ workId }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const [body, setBody] = useState('')
  const [tags, setTags] = useState('')

  const notes = useQuery({
    queryKey: [...keys.notes, workId],
    queryFn: () => listNotes({ work_id: workId }),
  })

  const settle = () => {
    void client.invalidateQueries({ queryKey: keys.notes })
    // A new tag on a note changes the tag list the rest of the app reads.
    void client.invalidateQueries({ queryKey: keys.tags })
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

  const remove = useMutation({
    mutationFn: deleteNote,
    onSuccess: () => {
      settle()
      say.ok(t('toast.noteDeleted'))
    },
    onError: (cause) => say.failedTo(t('toast.noteSaveFailed'), cause),
  })

  return (
    <section className="flex flex-col gap-3">
      <h3 className="text-sm font-semibold">{t('notes.title')}</h3>

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
                <p className="whitespace-pre-wrap text-sm">{note.body}</p>
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
              <Button
                variant="danger"
                size="iconSm"
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
