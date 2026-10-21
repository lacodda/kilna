import { useMemo, useState } from 'react'
import { useNavigate, useParams, useSearchParams } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { ListChecks, Plus } from 'lucide-react'
import { createNote } from '@/lib/api/notes'
import type { Note, NoteKind, NoteState } from '@/lib/api/types'
import { progressOf } from '@/lib/checklist'
import { isLine, isMaterial, NOTE_STATES, titleOf } from '@/lib/notes'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { plainNoteKindsOf } from '@/lib/canon'
import { labelOf, say as sayLabel, useProfile } from '@/lib/useProfile'
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
import { LineBank } from '@/features/notes/LineBank'
import { NoteDetail } from '@/features/notes/NoteDetail'
import { TagMenu } from '@/features/notes/TagMenu'

/** Where the kind on show is kept in the address: `/notes?kind=phrase`. */
const KIND = 'kind'

/** The state filter's "every state", beside the four states. */
const ANY_STATE = 'any'

/**
 * The material for works to come, on two shelves (ADR 0064): the phrases - a
 * word, a line, a quote, whatever turns up in a head as one line - and the
 * notes, a thought from a seed to a written page. A chip each, one kind on
 * show at a time.
 *
 * Until v0.76 a note could be written only from a work's card and read only
 * there, which is why the notes that were not about a work had nowhere to
 * live, and why editing one was impossible anywhere: `update_note` had no
 * caller. This is their home; the card's tab stays as the view from one work.
 *
 * Until v0.98 there were four chips - notes, ideas, phrases and a bank of
 * words - and an idea differed from a note by one flag, a word from a phrase
 * by being a row of another table. A person could not tell them apart, so
 * they are two.
 *
 * The kind on show and the open note are part of the address, so the back
 * button walks between them and a proposal or a search hit can land on one
 * directly. A kind of one line - a phrase - is a row each, with its shelves
 * (ADR 0045); a kind that is material is read by its state, fresh first.
 */
export function NotesView() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { config } = useProfile()
  const { noteId } = useParams()
  const [params] = useSearchParams()
  // A card of the canon is a note too, but it lives on the Canon screen
  // (ADR 0043): this list is the plain notes, and so are its kinds.
  const kinds = plainNoteKindsOf(config)
  // Every note of the profile: the chips' counts, and the kind of a note the
  // address opens without naming one.
  const everything = useQuery(queries.notesMatching({ canon: false }))
  const asked =
    params.get(KIND) ?? (everything.data ?? []).find((note) => note.id === noteId)?.kind ?? null
  // The kind on show: the one the address names, or the craft's first.
  const kind = kinds.find((one) => one.key === asked)?.key ?? kinds[0]?.key
  const setKind = (next: string) => {
    void navigate(`/notes?${new URLSearchParams({ [KIND]: next }).toString()}`)
  }

  const [state, setState] = useState<NoteState | typeof ANY_STATE>('fresh')
  const [tag, setTag] = useState('')
  const [text, setText] = useState('')
  // The list is a query; typing into it unthrottled would refetch per letter.
  const query = useDebounced(text.trim(), 200)
  // A note just made opens in the editor rather than as an empty page.
  const [fresh, setFresh] = useState<string | null>(null)

  const material = isMaterial(config, kind)
  const line = isLine(config, kind)
  const filter = {
    kind,
    tag: tag === '' ? undefined : tag,
    search: query === '' ? undefined : query,
    canon: false,
    state: material && state !== ANY_STATE ? state : undefined,
  }
  const notes = useQuery({ ...queries.notesMatching(filter), enabled: !line })
  // Every tag in use, most used first: the filter's choices, and what the
  // tag field of the open note completes from.
  const tags = useQuery(queries.tags())
  const counts = useMemo(() => {
    const map = new Map<string, number>()
    for (const note of everything.data ?? []) map.set(note.kind, (map.get(note.kind) ?? 0) + 1)
    return map
  }, [everything.data])

  const open = (id: string | null) => {
    const search = kind === undefined ? '' : `?${KIND}=${encodeURIComponent(kind)}`
    void navigate(id === null ? `/notes${search}` : `/notes/${id}${search}`)
  }

  const add = useAppMutation({
    mutationFn: () =>
      createNote({
        body: '',
        // Made in the kind being looked at, so it does not vanish from the
        // list it was made in.
        kind,
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
  const total = kind === undefined ? 0 : (counts.get(kind) ?? 0)
  const filtered = tag !== '' || query !== '' || (material && state !== ANY_STATE)

  return (
    <Frame
      head={
        <>
          {/* The kinds as chips, one always on: the two shelves are two
              screens of one place, not a filter that can be let go of. */}
          <ChipGroup
            aria-label={t('notes.kind')}
            value={kind === undefined ? [] : [kind]}
            onValueChange={(next) => {
              if (next[0] !== undefined) setKind(next[0])
            }}
          >
            {kinds.map((one) => (
              <Chip key={one.key} value={one.key} count={counts.get(one.key) ?? 0}>
                {labelOf(kinds, one.key)}
              </Chip>
            ))}
          </ChipGroup>
          {/* Where the material stands: fresh first, because what is still
              there to use is what a bank is opened for. */}
          {material && (
            <Select
              value={state}
              onChange={(next) =>
                setState(next === '' ? 'fresh' : (next as NoteState | typeof ANY_STATE))
              }
              options={[
                ...NOTE_STATES.map((one) => ({ value: one, label: t(`notes.states.${one}`) })),
                { value: ANY_STATE, label: t('notes.anyState') },
              ]}
              aria-label={t('notes.stateLabel')}
              className="w-44"
            />
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
          {/* The tag on show can be renamed, or taken off every note. */}
          {tag !== '' && <TagMenu tag={tag} onTag={setTag} />}
          {!line && kind !== undefined && (
            <Button
              variant="primary"
              className="ml-auto"
              disabled={add.isPending}
              onClick={() => add.mutate()}
            >
              <Plus aria-hidden />
              {t('notes.new')}
            </Button>
          )}
        </>
      }
    >
      {line && kind !== undefined ? (
        <LineBank kind={kind} filter={filter} tag={tag} filtered={filtered} total={total} />
      ) : (
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
                          library={kinds.find(
                            (one) => one.key === note.kind && one.library === true,
                          )}
                          material={isMaterial(config, note.kind)}
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
      )}
    </Frame>
  )
}

function NoteRow({
  note,
  kindLabel,
  library,
  material,
  active,
  onOpen,
}: {
  note: Note
  kindLabel: string
  /** The library's kind, when the note is an entry of it. */
  library?: NoteKind
  /** Whether it is spent by works, and so says where it stands. */
  material: boolean
  active: boolean
  onOpen: () => void
}) {
  const { t } = useTranslation()
  const title = titleOf(note)
  const progress = progressOf(note.body)
  const day = formatDay(note.updated_at)
  const state = material && note.state !== 'fresh' ? ` · ${t(`notes.state.${note.state}`)}` : ''
  // An entry of the library reads by who made it, when, and in what - not
  // by the kind every row on the screen shares.
  const made = [
    note.attribution,
    note.year,
    library?.media?.find((medium) => medium.key === note.medium)?.label,
  ]
    .map((part) => (part == null ? '' : typeof part === 'string' ? part : sayLabel(part)))
    .filter((part) => part.trim() !== '')
    .join(' · ')

  return (
    <RowButton
      selected={active}
      onClick={onOpen}
      description={library !== undefined && made !== '' ? made : `${kindLabel}${state} · ${day}`}
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
