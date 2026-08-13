import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { X } from 'lucide-react'
import { createNote, deleteNote, listNotes, type Note } from '@/lib/api'
import { Button } from '@/components/ui/Button'
import { Input, Textarea } from '@/components/ui/Input'

interface Props {
  workId: string
}

export function NotePanel({ workId }: Props) {
  const { t } = useTranslation()
  const [notes, setNotes] = useState<Note[]>([])
  const [body, setBody] = useState('')
  const [tags, setTags] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [revision, setRevision] = useState(0)

  useEffect(() => {
    listNotes({ work_id: workId })
      .then((result) => {
        setNotes(result)
        setError(null)
      })
      .catch((cause: unknown) => setError(String(cause)))
  }, [workId, revision])

  const add = () => {
    if (body.trim() === '') return

    createNote({
      body: body.trim(),
      work_id: workId,
      // Comma-separated in the field, an array in the store.
      tags: tags
        .split(',')
        .map((tag) => tag.trim())
        .filter((tag) => tag !== ''),
    })
      .then(() => {
        setBody('')
        setTags('')
        setRevision((current) => current + 1)
      })
      .catch((cause: unknown) => setError(String(cause)))
  }

  return (
    <section className="flex flex-col gap-3">
      <h3 className="text-sm font-semibold">{t('notes.title')}</h3>

      {error !== null && (
        <p role="alert" className="text-sm text-bad">
          {error}
        </p>
      )}

      {notes.length > 0 && (
        <ul className="flex flex-col gap-2">
          {notes.map((note) => (
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
                onClick={() => {
                  deleteNote(note.id)
                    .then(() => setRevision((current) => current + 1))
                    .catch((cause: unknown) => setError(String(cause)))
                }}
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
          add()
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
          <Button type="submit" variant="primary" disabled={body.trim() === ''}>
            {t('notes.add')}
          </Button>
        </div>
      </form>
    </section>
  )
}
