import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import { StickyNote } from 'lucide-react'
import type { Note, Work } from '@/lib/api/types'
import { createNote } from '@/lib/api/notes'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { documentKindsOf } from '@/features/work/tabs/overview/ReviewsWidget'
import { Widget } from '@/features/work/tabs/overview/Widget'

/** How many notes the widget lists before it hands over to the tab. */
const NOTES_SHOWN = 4

/** The notes about a work that are not its documents, the newest first. */
function plainNotesOf(notes: readonly Note[], documents: readonly string[]): Note[] {
  return notes
    .filter((note) => !documents.includes(note.kind))
    .sort((a, b) => b.updated_at.localeCompare(a.updated_at))
}

/** A note in one line: its title, or where its text begins. */
function lineOf(note: Note): string {
  const title = note.title?.trim() ?? ''
  if (title !== '') return title
  return note.body.split('\n').find((line) => line.trim() !== '') ?? ''
}

/**
 * The notes about a work that goes out, and the place to write the first one
 * (v0.166).
 *
 * A clip, an audio release and a short are not prepared on a conception, so
 * the tab that holds a work's notes is not drawn on their cards until there
 * is a note to hold - and the only way to attach the first was the Notes
 * screen's "Attach to a work", a screen away from the thing the note was
 * about. The widget is that place: a line to write, and under it what is
 * written already, each row the way to the notes themselves.
 */
export function NotesWidget({ work }: { work: Work }) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { config } = useProfile()
  const [line, setLine] = useState('')
  const attached = useQuery(queries.notesMatching({ work_id: work.id }))
  const notes = plainNotesOf(attached.data ?? [], documentKindsOf(config))

  const add = useAppMutation({
    mutationFn: (body: string) => createNote({ body, work_id: work.id }),
    failure: 'overview.noteAddFailed',
    refresh: refresh.note,
    onSuccess: () => setLine(''),
  })
  const submit = () => {
    const body = line.trim()
    if (body !== '' && !add.isPending) add.mutate(body)
  }

  const more = notes.length - NOTES_SHOWN
  return (
    <Widget caption={t('overview.notes')}>
      <div className="flex min-w-0 items-center gap-2">
        <Input
          value={line}
          placeholder={t('overview.notePlaceholder')}
          aria-label={t('overview.noteAdd')}
          onChange={(event) => setLine(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === 'Enter') submit()
          }}
        />
        <Button
          size="sm"
          variant="soft"
          disabled={add.isPending || line.trim() === ''}
          onClick={submit}
        >
          {t('overview.noteAdd')}
        </Button>
      </div>
      {notes.length > 0 && (
        <ul className="flex flex-col">
          {notes.slice(0, NOTES_SHOWN).map((note) => (
            <li key={note.id}>
              <Button
                variant="icon"
                size={null}
                onClick={() => void navigate(`/works/${work.id}/conception`)}
                className="w-full justify-start gap-2 px-1.5 py-1 text-left font-normal"
              >
                <StickyNote aria-hidden className="size-3.5 shrink-0 text-faint" />
                <span className="min-w-0 flex-1 truncate text-sm text-text">
                  {lineOf(note) || t('notes.untitled')}
                </span>
              </Button>
            </li>
          ))}
        </ul>
      )}
      {more > 0 && (
        <p className="text-xs">
          <Button variant="link" onClick={() => void navigate(`/works/${work.id}/conception`)}>
            {t('overview.notesMore', { count: more })}
          </Button>
        </p>
      )}
    </Widget>
  )
}
