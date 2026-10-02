import { useMemo, useState } from 'react'
import { useNavigate, useParams, useSearchParams } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { ListChecks, Plus } from 'lucide-react'
import { createNote } from '@/lib/api/notes'
import type { Note, NoteState } from '@/lib/api/types'
import { progressOf } from '@/lib/checklist'
import { isLine, isMaterial, NOTE_STATES, titleOf } from '@/lib/notes'
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
import { LineBank } from '@/features/notes/LineBank'
import { NoteDetail } from '@/features/notes/NoteDetail'
import { WordBank } from '@/features/words/WordBank'

/*
 * What a kind chip is called in its group. "All" is a chip of its own, and the
 * kinds carry a prefix, so a craft that names a kind "all" cannot be taken for
 * it.
 */
const ALL_KINDS = 'all'
/** The chip of the bank of words: a view of its own, not a kind of note. */
const WORDS = 'words'
const chipOf = (kind: string | undefined) => (kind === undefined ? ALL_KINDS : `kind:${kind}`)
const kindOf = (chip: string | undefined) =>
  chip === undefined || chip === ALL_KINDS || chip === WORDS
    ? undefined
    : chip.slice('kind:'.length)

/** The state filter's "every state", beside the four states. */
const ANY_STATE = 'any'

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
 *
 * A kind of one line - a phrase - is not a page among the others: its chip
 * opens the bank of lines instead, a row each, and "All" leaves it out, so a
 * thousand phrases never bury the notes (ADR 0045). A kind that is material -
 * an idea, a phrase - is read by its state, fresh first.
 *
 * The bank of words has a chip of its own beside the kinds, though a word is
 * no note: it is a word of the record (ADR 0052), kept here because this is
 * where the material for songs to come is. It is in the address (`?words`),
 * the way the canon's timeline is, so a proposal of words can open on it.
 */
export function NotesView() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { config } = useProfile()
  const { noteId } = useParams()
  const [params] = useSearchParams()
  const words = params.has(WORDS)
  // A card of the canon is a note too, but it lives on the Canon screen
  // (ADR 0043): this list is the plain notes, and so are its kinds.
  const kinds = plainNoteKindsOf(config)

  const [kind, setKind] = useState<string | undefined>(undefined)
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
    // "All" is the pages: the lines have a bank of their own.
    line: kind === undefined ? false : undefined,
    state: material && state !== ANY_STATE ? state : undefined,
  }
  const notes = useQuery({ ...queries.notesMatching(filter), enabled: !line && !words })
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
  // The bank's chip counts the words in it, as a kind's counts its notes.
  const terms = useQuery(queries.terms())
  const banked = (terms.data ?? []).filter((term) => term.bank !== null).length

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
  // "All" counts what "All" shows: the pages, not the lines.
  const total = (everything.data ?? []).filter((note) => !isLine(config, note.kind)).length
  const filtered =
    kind !== undefined || tag !== '' || query !== '' || (material && state !== ANY_STATE)

  return (
    <Frame
      head={
        <>
          {/* The kinds as chips, the way the style dictionary narrows to a type:
              "every character" is one click, and the chip that is on turns off -
              letting go of one leaves the group empty, which is every kind.
              A craft that names no kinds still has "All" and the bank of
              words to choose between. */}
          <ChipGroup
            aria-label={t('notes.kind')}
            value={[words ? WORDS : chipOf(kind)]}
            onValueChange={(next) => {
              if (next[0] === WORDS) {
                void navigate(`/notes?${WORDS}`)
                return
              }
              if (words) void navigate('/notes')
              setKind(kindOf(next[0]))
            }}
          >
            <Chip value={chipOf(undefined)} count={total}>
              {t('notes.allKinds')}
            </Chip>
            {kinds.map((one) => (
              <Chip key={one.key} value={chipOf(one.key)} count={counts.get(one.key) ?? 0}>
                {labelOf(kinds, one.key)}
              </Chip>
            ))}
            <Chip value={WORDS} count={banked}>
              {t('words.view')}
            </Chip>
          </ChipGroup>
          {/* Where the material stands: fresh first, because what is still
              there to use is what a bank is opened for. */}
          {material && !words && (
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
          {!words && (
            <Input
              value={text}
              onChange={(event) => setText(event.target.value)}
              placeholder={t('notes.search')}
              aria-label={t('notes.search')}
              className="w-56"
            />
          )}
          {!words && (tags.data ?? []).length > 0 && (
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
          {!line && !words && (
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
      {words ? (
        <WordBank />
      ) : line && kind !== undefined ? (
        <LineBank kind={kind} filter={filter} tag={tag} filtered={filtered} />
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
  material,
  active,
  onOpen,
}: {
  note: Note
  kindLabel: string
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

  return (
    <RowButton
      selected={active}
      onClick={onOpen}
      description={`${kindLabel}${state} · ${day}`}
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
