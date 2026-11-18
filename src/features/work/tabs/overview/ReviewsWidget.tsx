import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import { FileText } from 'lucide-react'
import type { Note, ProfileConfig, Work } from '@/lib/api/types'
import { formatDay } from '@/lib/format'
import { queries } from '@/lib/query/queries'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Widget } from '@/features/work/tabs/overview/Widget'

/** The kinds of note that are long documents, as the profile names them. */
export function documentKindsOf(config: Pick<ProfileConfig, 'note_kinds'>): string[] {
  return config.note_kinds.filter((kind) => kind.document === true).map((kind) => kind.key)
}

/** The documents attached to a work, the newest edited first. */
export function documentsOf(notes: readonly Note[], kinds: readonly string[]): Note[] {
  return notes
    .filter((note) => kinds.includes(note.kind))
    .sort((a, b) => b.updated_at.localeCompare(a.updated_at))
}

/**
 * What has been written about the work at length (v0.147, FR-10 10.4): a
 * review of a video, the list of its edits, any document attached to it -
 * each a row that opens in the reader. Drawn only on a work that holds one:
 * the reviews of a channel's videos were notes nobody found from the video.
 */
export function ReviewsWidget({ work }: { work: Work }) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { config } = useProfile()
  const notes = useQuery(queries.notesMatching({ work_id: work.id }))
  const kinds = documentKindsOf(config)
  const documents = documentsOf(notes.data ?? [], kinds)
  const kindOf = (key: string) => config.note_kinds.find((kind) => kind.key === key)

  return (
    <Widget caption={t('overview.reviews')}>
      <ul className="flex flex-col">
        {documents.map((note) => (
          <li key={note.id}>
            <Button
              variant="icon"
              size={null}
              onClick={() =>
                void navigate(
                  `/notes/${note.id}?${new URLSearchParams({ kind: note.kind }).toString()}`,
                )
              }
              className="w-full justify-start gap-2 px-1.5 py-1 text-left font-normal"
            >
              <FileText aria-hidden className="size-3.5 shrink-0 text-faint" />
              <span className="min-w-0 flex-1 truncate text-sm text-text">
                {note.title ?? t('notes.untitled')}
              </span>
              <span className="shrink-0 text-2xs text-faint">
                {sayLabel(kindOf(note.kind)?.label ?? note.kind)} · {formatDay(note.updated_at)}
              </span>
            </Button>
          </li>
        ))}
      </ul>
    </Widget>
  )
}
