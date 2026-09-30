import { useMemo, useState } from 'react'
import { useNavigate, useParams } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { ListChecks, Plus } from 'lucide-react'
import { createNote } from '@/lib/api/notes'
import type { Note } from '@/lib/api/types'
import { progressOf } from '@/lib/checklist'
import { titleOf } from '@/lib/notes'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { plainNoteKindsOf } from '@/lib/canon'
import { labelOf, useProfile } from '@/lib/useProfile'
import { useDebounced } from '@/lib/useDebounced'
import { formatDay } from '@/lib/format'
import { Button } from '@/components/ui/button'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { Input } from '@/components/ui/input'
import { RowButton } from '@/components/ui/list-row'
import { Select } from '@/components/AppSelect'
import { EmptyState } from '@/components/ui/empty-state'
import { SkeletonList } from '@/components/ui/skeleton'
import { Frame, ListDetail, Pane } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { NoteDetail } from '@/features/notes/NoteDetail'

/*
 * What a kind chip is called in its group. "All" is a chip of its own, and the
 * kinds carry a prefix, so a craft that names a kind "all" cannot be taken for
 * it.
 */
const ALL_KINDS = 'all'
const chipOf = (kind: string | undefined) => (kind === undefined ? ALL_KINDS : `kind:${kind}`)
const kindOf = (chip: string | undefined) =>
  chip === undefined || chip === ALL_KINDS ? undefined : chip.slice('kind:'.length)

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
  const { config } = useProfile()
  const { noteId } = useParams()
  // A card of the canon is a note too, but it lives on the Canon screen
  // (ADR 0043): this list is the plain notes, and so are its kinds.
  const kinds = plainNoteKindsOf(config)

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
    canon: false,
  }
  const notes = useQuery(queries.notesMatching(filter))
  // Every tag in use, most used first: the filter's choices, and what the
  // tag field of the open note completes from.
  const tags = useQuery(queries.tags())
  // Counts per kind come from the unfiltered list, so a chip says how many
  // there are of that kind, not how many survived the other filters.
  const everything = useQuery(queries.notesMatching({ canon: false }))
  const counts = useMemo(() => {
    const map = new Map<string, number>()
    for (const note of everything.data ?? []) map.set(note.kind, (map.get(note.kind) ?? 0) + 1)
    return map
  }, [everything.data])

  const open = (id: string | null) => {
    void navigate(id === null ? '/notes' : `/notes/${id}`)
  }

  const add = useAppMutation({
    mutationFn: () =>
      createNote({
        body: '',
        // Made in the kind being looked at, so it does not vanish from the
        // list it was made in.
        kind: kind ?? kinds[0]?.key,
        tags: tag === '' ? [] : [tag],
      }),
    failure: 'toast.noteSaveFailed',
    refresh: refresh.note,
    onSuccess: (created) => {
      setFresh(created.id)
      open(created.id)
    },
  })

  const rows = notes.data ?? []
  const selected: Note | undefined =
    rows.find((note) => note.id === noteId) ??
    (everything.data ?? []).find((note) => note.id === noteId)
  const total = everything.data?.length ?? 0
  const filtered = kind !== undefined || tag !== '' || query !== ''

  return (
    <Frame
      head={
        <>
          {/* The kinds as chips, the way the style dictionary narrows to a type:
              "every character" is one click, and the chip that is on turns off -
              letting go of one leaves the group empty, which is every kind.
              Only when the craft names kinds — one that names none has one. */}
          {kinds.length > 0 && (
            <ChipGroup
              aria-label={t('notes.kind')}
              value={[chipOf(kind)]}
              onValueChange={(next) => setKind(kindOf(next[0]))}
            >
              <Chip value={chipOf(undefined)} count={total}>
                {t('notes.allKinds')}
              </Chip>
              {kinds.map((one) => (
                <Chip key={one.key} value={chipOf(one.key)} count={counts.get(one.key) ?? 0}>
                  {labelOf(kinds, one.key)}
                </Chip>
              ))}
            </ChipGroup>
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
        </>
      }
    >
      <ListDetail
        list={
          <Pane label={t('nav.notes')} bodyClassName="p-1.5">
            <Loaded
              query={notes}
              skeleton={<SkeletonList rows={6} />}
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
            </Loaded>
          </Pane>
        }
        detail={
          selected !== undefined ? (
            <NoteDetail
              key={selected.id}
              note={selected}
              tags={(tags.data ?? []).map(([name]) => name)}
              startEditing={fresh === selected.id}
              onTag={(name) => setTag(name)}
              onGone={() => open(null)}
            />
          ) : noteId !== undefined && !notes.isPending && !everything.isPending ? (
            <EmptyState title={t('notes.gone')} body={t('notes.goneBody')} className="flex-1" />
          ) : (
            <EmptyState
              className="flex-1"
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
          )
        }
      />
    </Frame>
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
  const { t } = useTranslation()
  const title = titleOf(note)
  const progress = progressOf(note.body)
  const day = formatDay(note.updated_at)

  return (
    <RowButton
      selected={active}
      onClick={onOpen}
      description={`${kindLabel} · ${day}`}
      // How far along a checklist is, where the note has one: the reason to
      // open a to-do list is usually to see what is left.
      end={
        progress.total > 0 ? (
          <span className="flex items-center gap-0.5">
            <ListChecks aria-hidden className="size-3" />
            {progress.done}/{progress.total}
          </span>
        ) : undefined
      }
    >
      {title === '' ? <span className="text-faint">{t('notes.untitled')}</span> : title}
    </RowButton>
  )
}
