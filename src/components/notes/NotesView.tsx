import { useMemo, useState } from 'react'
import { useNavigate, useParams } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ListChecks, Plus } from 'lucide-react'
import { createNote, listNotes, listTags, type Note } from '@/lib/api'
import { progressOf } from '@/lib/checklist'
import { titleOf } from '@/lib/notes'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { labelOf, useProfile } from '@/lib/useProfile'
import { useDebounced } from '@/lib/useDebounced'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Select } from '@/components/ui/AppSelect'
import { EmptyState } from '@/components/ui/EmptyState'
import { SkeletonList } from '@/components/ui/Skeleton'
import { NoteDetail } from '@/components/notes/NoteDetail'

/**
 * Every note of the profile in one place: the list on the left with its own
 * row of filters, the open note on the right, its tags along the bottom.
 *
 * Until v0.76 a note could be written only from a work's card and read only
 * there, which is why the notes that were not about a work — an idea, a
 * piece of lore, a reference — had nowhere to live, and why editing one was
 * impossible anywhere: `update_note` had no caller. This is their home; the
 * card's tab stays as the view from one work.
 *
 * The open note is part of the address, so the back button walks between
 * notes and a search hit can land on one directly.
 */
export function NotesView() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const client = useQueryClient()
  const { config } = useProfile()
  const { noteId } = useParams()
  const kinds = config.note_kinds ?? []

  const [kind, setKind] = useState<string | undefined>(undefined)
  const [tag, setTag] = useState('')
  const [text, setText] = useState('')
  // The list is a query; typing into it unthrottled would refetch per letter.
  const query = useDebounced(text.trim(), 200)
  // A note just made opens in the editor rather than as an empty page.
  const [fresh, setFresh] = useState<string | null>(null)

  const filter = {
    kind,
    tag: tag === '' ? undefined : tag,
    search: query === '' ? undefined : query,
  }
  const notes = useQuery({
    queryKey: [...keys.notes, 'all', filter],
    queryFn: () => listNotes(filter),
  })
  // Every tag in use, most used first: the filter's choices, and what the
  // tag field of the open note completes from.
  const tags = useQuery({ queryKey: keys.tags, queryFn: listTags })
  // Counts per kind come from the unfiltered list, so a chip says how many
  // there are of that kind, not how many survived the other filters.
  const everything = useQuery({
    queryKey: [...keys.notes, 'all', {}],
    queryFn: () => listNotes({}),
  })
  const counts = useMemo(() => {
    const map = new Map<string, number>()
    for (const note of everything.data ?? []) map.set(note.kind, (map.get(note.kind) ?? 0) + 1)
    return map
  }, [everything.data])

  const open = (id: string | null) => {
    void navigate(id === null ? '/notes' : `/notes/${id}`)
  }

  const add = useMutation({
    mutationFn: () =>
      createNote({
        body: '',
        // Made in the kind being looked at, so it does not vanish from the
        // list it was made in.
        kind: kind ?? kinds[0]?.key ?? null,
        tags: tag === '' ? [] : [tag],
      }),
    onSuccess: (created) => {
      for (const key of [keys.notes, keys.tags, keys.journal]) {
        void client.invalidateQueries({ queryKey: key })
      }
      setFresh(created.id)
      open(created.id)
    },
    onError: (cause) => say.failedTo(t('toast.noteSaveFailed'), cause),
  })

  const rows = notes.data ?? []
  const selected: Note | undefined =
    rows.find((note) => note.id === noteId) ??
    (everything.data ?? []).find((note) => note.id === noteId)
  const total = everything.data?.length ?? 0
  const filtered = kind !== undefined || tag !== '' || query !== ''

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3">
      <div className="flex shrink-0 flex-wrap items-center gap-2">
        {/* The kinds as chips, the way the style dictionary narrows to a type:
            "every character" is one click, and the chip that is on turns off.
            Only when the craft names kinds — one that names none has one. */}
        {kinds.length > 0 && (
          <div role="group" aria-label={t('notes.kind')} className="flex flex-wrap items-center gap-1.5">
            {[
              { key: undefined, label: t('notes.allKinds'), count: total },
              ...kinds.map((one) => ({
                key: one.key as string | undefined,
                label: labelOf(kinds, one.key),
                count: counts.get(one.key) ?? 0,
              })),
            ].map((entry) => {
              const active = kind === entry.key
              return (
                <button
                  key={entry.key ?? ''}
                  type="button"
                  aria-pressed={active}
                  onClick={() => setKind(active ? undefined : entry.key)}
                  className={cn(
                    'flex cursor-pointer items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-[11.5px] transition-colors',
                    active
                      ? 'border-transparent bg-accent-soft font-semibold text-accent-2'
                      : 'border-line text-dim hover:border-line-2 hover:text-text',
                  )}
                >
                  {entry.label}
                  <span className="text-[10.5px] text-faint tabular-nums">{entry.count}</span>
                </button>
              )
            })}
          </div>
        )}
        <Input
          value={text}
          onChange={(event) => setText(event.target.value)}
          placeholder={t('notes.search')}
          aria-label={t('notes.search')}
          className="w-56"
        />
        {(tags.data ?? []).length > 0 && (
          <Select
            value={tag}
            onChange={setTag}
            placeholder={t('notes.anyTag')}
            options={(tags.data ?? []).map(([name, uses]) => ({
              value: name,
              label: `${name} · ${String(uses)}`,
            }))}
            aria-label={t('notes.tag')}
            className="w-44"
          />
        )}
        <Button
          variant="primary"
          className="ml-auto"
          disabled={add.isPending}
          onClick={() => add.mutate()}
        >
          <Plus aria-hidden />
          {t('notes.new')}
        </Button>
      </div>

      <div className="grid min-h-0 flex-1 grid-cols-[270px_minmax(0,1fr)] gap-3">
        <div className="flex min-h-0 flex-col overflow-hidden rounded-xl border border-line bg-raise">
          <div className="min-h-0 flex-1 overflow-y-auto p-1.5">
            {notes.isPending ? (
              <SkeletonList rows={6} />
            ) : notes.isError ? (
              <p role="alert" className="p-3 text-sm text-bad">
                {t('toast.loadFailed')}
              </p>
            ) : rows.length === 0 ? (
              <p className="p-3 text-xs text-faint">
                {filtered ? t('notes.noMatches') : t('notes.none')}
              </p>
            ) : (
              <ul className="flex flex-col gap-0.5">
                {rows.map((note) => (
                  <li key={note.id}>
                    <NoteRow
                      note={note}
                      kindLabel={labelOf(kinds, note.kind)}
                      active={note.id === noteId}
                      onOpen={() => open(note.id)}
                    />
                  </li>
                ))}
              </ul>
            )}
          </div>
        </div>

        {selected !== undefined ? (
          <NoteDetail
            key={selected.id}
            note={selected}
            tags={(tags.data ?? []).map(([name]) => name)}
            startEditing={fresh === selected.id}
            onTag={(name) => setTag(name)}
            onGone={() => open(null)}
          />
        ) : noteId !== undefined && !notes.isPending && !everything.isPending ? (
          <EmptyState title={t('notes.gone')} body={t('notes.goneBody')} />
        ) : (
          <EmptyState
            title={total === 0 ? t('notes.empty') : t('notes.pick')}
            body={total === 0 ? t('notes.emptyBody') : undefined}
            action={
              total === 0 ? (
                <Button variant="primary" onClick={() => add.mutate()}>
                  <Plus aria-hidden />
                  {t('notes.new')}
                </Button>
              ) : undefined
            }
          />
        )}
      </div>
    </div>
  )
}

function NoteRow({
  note,
  kindLabel,
  active,
  onOpen,
}: {
  note: Note
  kindLabel: string
  active: boolean
  onOpen: () => void
}) {
  const { t, i18n } = useTranslation()
  const title = titleOf(note)
  const progress = progressOf(note.body)
  const day = new Date(note.updated_at).toLocaleDateString(i18n.language, {
    day: 'numeric',
    month: 'short',
  })

  return (
    <button
      type="button"
      onClick={onOpen}
      aria-current={active ? 'true' : undefined}
      className={cn(
        'flex w-full cursor-pointer flex-col gap-0.5 rounded-lg px-2.5 py-2 text-left transition-colors',
        active ? 'bg-accent-soft' : 'hover:bg-soft',
      )}
    >
      <b className={cn('block truncate text-[12.5px] font-semibold', title === '' && 'text-faint')}>
        {title === '' ? t('notes.untitled') : title}
      </b>
      <span className="flex min-w-0 items-center gap-1.5 text-[11px] text-faint">
        <span className="truncate">
          {kindLabel} · {day}
        </span>
        {/* How far along a checklist is, where the note has one: the reason
            to open a to-do list is usually to see what is left. */}
        {progress.total > 0 && (
          <span className="ml-auto flex shrink-0 items-center gap-0.5 tabular-nums">
            <ListChecks aria-hidden className="size-3" />
            {progress.done}/{progress.total}
          </span>
        )}
      </span>
    </button>
  )
}
