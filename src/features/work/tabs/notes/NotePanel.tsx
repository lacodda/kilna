import { useState } from 'react'
import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { ArrowUpRight, X } from 'lucide-react'
import { createNote, deleteNote, updateNote } from '@/lib/api/notes'
import { cardKindOf } from '@/lib/canon'
import { toggleTask } from '@/lib/checklist'
import { isLine } from '@/lib/notes'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { announceDeleted } from '@/lib/trash'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Chip } from '@/components/ui/chip'
import { Input } from '@/components/ui/input'
import { Markdown } from '@/components/Markdown'
import { Panel, SectionLabel, panelVariants } from '@/components/ui/panel'
import { Textarea } from '@/components/ui/textarea'
import { EmptyState } from '@/components/ui/empty-state'
import { SkeletonList } from '@/components/ui/skeleton'
import { Frame, Scroll } from '@/components/frame'
import { Loaded } from '@/components/Loaded'

interface Props {
  workId: string
}

/**
 * The notes about one work, on its card: the notes and the phrases kept for
 * it (ADR 0064), and the heroes of the canon who live at it.
 *
 * The mockup's anatomy: a caption over the tab, each note a panel of its own
 * with its tags as chips under the text, and the form for a new one in a
 * panel at the foot. A work of a kind prepared on a conception holds the same
 * list as one part of its Conception tab (`AttachedNotes`); a work of any
 * other kind has this alone (v0.99).
 */
export function NotePanel({ workId }: Props) {
  const { t } = useTranslation()
  // The form for a new note stands at the foot of the tab, where the eye
  // ends up after reading the notes above it - it used to follow the last
  // note down, and a long list pushed it off the card.
  return (
    <Frame
      head={<SectionLabel>{t('conception.attached')}</SectionLabel>}
      foot={
        <Panel className="w-full px-3 py-2.5">
          <NoteForm workId={workId} />
        </Panel>
      }
    >
      <Scroll label={t('conception.attached')}>
        <NoteList workId={workId} />
      </Scroll>
    </Frame>
  )
}

/**
 * The notes and phrases attached to a work as a part of its Conception tab:
 * the same list and form, under their heading, in the tab's own scroll.
 */
export function AttachedNotes({ workId }: Props) {
  const { t } = useTranslation()
  return (
    <section aria-label={t('conception.attached')} className="flex flex-col gap-2">
      <SectionLabel>{t('conception.attached')}</SectionLabel>
      <NoteList workId={workId} />
      <Panel className="w-full px-3 py-2.5">
        <NoteForm workId={workId} />
      </Panel>
    </section>
  )
}

/** A new note for the work: its words, and its tags. */
function NoteForm({ workId }: Props) {
  const { t } = useTranslation()
  const [body, setBody] = useState('')
  const [tags, setTags] = useState('')

  const add = useAppMutation({
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
    failure: 'toast.noteSaveFailed',
    refresh: refresh.note,
    onSuccess: () => {
      setBody('')
      setTags('')
    },
  })

  return (
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
  )
}

/** The notes attached to the work, each a panel of its own. */
function NoteList({ workId }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const client = useQueryClient()
  const { config } = useProfile()

  const notes = useQuery(queries.notesFor(workId))

  // A box ticked here is the same edit as one ticked on the notes screen: the
  // body is rewritten and the box follows it.
  const tick = useAppMutation({
    mutationFn: ({ id, next }: { id: string; next: string }) => updateNote(id, { body: next }),
    failure: 'toast.noteSaveFailed',
    refresh: refresh.note,
  })

  // A deletion is refreshed by its announcement, which refreshes the same
  // areas again when it is taken back.
  const remove = useAppMutation({
    mutationFn: deleteNote,
    failure: 'toast.noteSaveFailed',
    onSuccess: (deletionId) =>
      announceDeleted({
        client,
        deletionId,
        message: t('toast.noteDeleted'),
        refresh: refresh.note,
      }),
  })

  return (
    <Loaded
      query={notes}
      skeleton={<SkeletonList rows={2} />}
      isEmpty={(data) => data.length === 0}
      // Plain, and its way out is the form under it.
      emptyState={<EmptyState plain title={t('notes.emptyForWork')} />}
      plain
    >
      {(data) => (
        <ul className="flex flex-col gap-2">
          {data.map((note) => (
            <li
              key={note.id}
              className={cn(panelVariants(), 'flex items-start gap-2.5 px-3 py-2.5')}
            >
              <div className="flex min-w-0 flex-1 flex-col gap-1.5">
                {/* A hero of this work is a card of the canon: named, and
                    opened where its facts are (ADR 0043). */}
                {cardKindOf(config, note.kind) !== undefined && (
                  <p className="flex items-center gap-1.5 text-xs">
                    <Chip variant="accent">{sayLabel(cardKindOf(config, note.kind)?.label)}</Chip>
                    <b className="font-semibold">{note.title}</b>
                  </p>
                )}
                {/* A phrase kept for the work says it is one: a line,
                    not a thought about the work. */}
                {isLine(config, note.kind) && (
                  <p className="flex items-center gap-1.5 text-xs">
                    <Chip>
                      {sayLabel(
                        (config.note_kinds ?? []).find((kind) => kind.key === note.kind)?.label,
                      )}
                    </Chip>
                    {note.state !== 'fresh' && (
                      <span className="text-faint">{t(`notes.state.${note.state}`)}</span>
                    )}
                  </p>
                )}
                {/* Rendered, not shown raw: a note is where a table of images
                    or a list of phrases lands, and pipes and asterisks are not
                    what its author wrote it to be read as. Line breaks inside a
                    paragraph are kept, as everywhere markdown is rendered here. */}
                <Markdown
                  body={note.body}
                  className="text-sm leading-relaxed text-dim"
                  onToggleTask={(index) =>
                    tick.mutate({ id: note.id, next: toggleTask(note.body, index) })
                  }
                />
                {note.attribution !== null && (
                  <p className="text-xs text-dim italic">— {note.attribution}</p>
                )}
                {note.tags.length > 0 && (
                  <p className="flex flex-wrap gap-1">
                    {note.tags.map((tag) => (
                      <Chip key={tag}>{tag}</Chip>
                    ))}
                  </p>
                )}
              </div>
              {/* Edited on the notes screen, where a note has room: the
                  card is the view from one work, not a second editor. */}
              <Button
                variant="icon"
                size="icon-sm"
                title={
                  cardKindOf(config, note.kind) !== undefined
                    ? t('canon.openInCanon')
                    : t('notes.openInNotes')
                }
                aria-label={
                  cardKindOf(config, note.kind) !== undefined
                    ? t('canon.openInCanon')
                    : t('notes.openInNotes')
                }
                onClick={() =>
                  void navigate(
                    cardKindOf(config, note.kind) !== undefined
                      ? `/canon/${note.id}`
                      : isLine(config, note.kind)
                        ? `/notes?kind=${encodeURIComponent(note.kind)}`
                        : `/notes/${note.id}?kind=${encodeURIComponent(note.kind)}`,
                  )
                }
              >
                <ArrowUpRight aria-hidden />
              </Button>
              <Button
                variant="danger"
                size="icon-sm"
                title={t('notes.delete')}
                aria-label={t('notes.delete')}
                onClick={() => remove.mutate(note.id)}
              >
                <X aria-hidden />
              </Button>
            </li>
          ))}
        </ul>
      )}
    </Loaded>
  )
}
