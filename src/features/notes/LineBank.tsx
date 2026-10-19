import { useMemo, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { addToShelf, deleteNote, keepLines, removeFromShelf, updateNote } from '@/lib/api/notes'
import type { LineRow, NoteFilter, NotePatch } from '@/lib/api/types'
import { ALL_LINES, linesIn, NO_SHELF, shelfOf, splitLines, type LineScope } from '@/lib/lines'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { announceDeleted } from '@/lib/trash'
import { say } from '@/lib/toast'
import { useChipDrag, type Dragging } from '@/lib/useChipDrag'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { SkeletonList } from '@/components/ui/skeleton'
import { Textarea } from '@/components/ui/textarea'
import { ListDetail, Pane } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { PickWorkDialog } from '@/components/PickWorkDialog'
import { LineItem } from '@/features/notes/LineItem'
import { PromoteNoteDialog } from '@/features/notes/PromoteNoteDialog'
import { Shelves } from '@/features/notes/Shelves'

/** How many rows are drawn at first, and how many more each "show more". */
const PAGE = 200

interface Props {
  /** The kind of line the screen keeps: a phrase. */
  kind: string
  /** The narrowing the screen's head already chose: state, tag, words. */
  filter: NoteFilter
  /** The tag the screen narrows to, which a new line is given. */
  tag: string
  /** Whether anything narrows the list, for the empty state's words. */
  filtered: boolean
  /** How many phrases there are at all: none is an empty screen, not a
   *  filter that left nothing. */
  total: number
}

/**
 * The phrases (ADR 0064): whatever turns up in a head as one line - a word, a
 * line, a quote - kept a row each, sorted onto shelves the owner names and
 * orders.
 *
 * Two screens became this one. The predecessor's 1,131 phrases lived as
 * cards nobody read past the third day, so a phrase is a row (ADR 0045); and
 * the bank of words kept single words apart, in blocks (ADR 0052), when a
 * word is only the shortest phrase. A row says what the record knows about
 * it: whose words it is, how it is sung, the shelves it stands on, how many
 * works already say it, the work it is kept for.
 *
 * Lines come in through the field at the top: one and Enter, or a list
 * pasted by lines - onto the shelf on show, or onto none. A phrase goes onto
 * a shelf by being carried there; that is a pointer drag (`useChipDrag`),
 * since a desktop shell that takes file drops never lets an HTML5 drag reach
 * the page.
 */
export function LineBank({ kind, filter, tag, filtered: narrowed, total }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const lines = useQuery(queries.lines(filter))
  const shelves = useQuery(queries.shelves())
  const catalogue = useQuery(queries.catalogue())
  const [scope, setScope] = useState<LineScope>(ALL_LINES)
  const [draft, setDraft] = useState('')
  const [shown, setShown] = useState(PAGE)
  const [attaching, setAttaching] = useState<LineRow | null>(null)
  const [promoting, setPromoting] = useState<LineRow | null>(null)
  // The shelf a carried phrase is over, lit while it is.
  const [over, setOver] = useState<string | null>(null)

  const rows = useMemo(() => lines.data ?? [], [lines.data])
  const list = useMemo(() => shelves.data ?? [], [shelves.data])
  // A shelf that went - deleted here, or elsewhere - leaves every phrase on
  // show rather than a list of nothing under a name that is not there.
  const current =
    scope === ALL_LINES || scope === NO_SHELF || shelfOf(scope, list) !== null ? scope : ALL_LINES
  const shelf = shelfOf(current, list)
  const visible = useMemo(() => linesIn(current, rows, list), [current, rows, list])
  const counts = useMemo(
    () => ({ all: rows.length, loose: linesIn(NO_SHELF, rows, list).length }),
    [rows, list],
  )
  const titles = useMemo(
    () => new Map((catalogue.data ?? []).map((row) => [row.work_id, row.title])),
    [catalogue.data],
  )
  const bodyOf = (id: string) => rows.find((row) => row.id === id)?.body ?? ''
  // A filter that left nothing is only that when there is something to filter.
  const filtered = narrowed && total > 0

  const keep = useAppMutation({
    mutationFn: (typed: string[]) =>
      keepLines(typed, kind, shelf?.id ?? null, tag === '' ? [] : [tag]),
    failure: 'notes.keepFailed',
    refresh: refresh.note,
    onSuccess: (kept) => {
      setDraft('')
      // A line kept already is not kept twice, and nothing on the screen
      // would say so: it stays as it stood, and joins the shelf on show.
      const known = kept.notes.length - kept.created
      if (known > 0) say.info(t('notes.known', { count: known }))
    },
  })
  const patch = useAppMutation({
    mutationFn: ({ id, change }: { id: string; change: NotePatch }) => updateNote(id, change),
    failure: 'toast.noteSaveFailed',
    refresh: refresh.note,
  })
  const put = useAppMutation({
    mutationFn: ({ shelfId, noteId }: { shelfId: string; noteId: string }) =>
      addToShelf(shelfId, noteId),
    failure: 'shelves.failed',
    refresh: refresh.note,
    // Said, because the phrase stays where it was on the right: the list on
    // show is still the one it was carried from.
    onSuccess: (_, { shelfId, noteId }) =>
      say.ok(t('shelves.put', { line: bodyOf(noteId), shelf: shelfOf(shelfId, list)?.name ?? '' })),
  })
  const takeOff = useAppMutation({
    mutationFn: ({ shelfId, noteId }: { shelfId: string; noteId: string }) =>
      removeFromShelf(shelfId, noteId),
    failure: 'shelves.failed',
    refresh: refresh.note,
  })
  const remove = useAppMutation({
    mutationFn: (id: string) => deleteNote(id),
    failure: 'toast.noteSaveFailed',
    onSuccess: (deletionId) =>
      announceDeleted({
        client,
        deletionId,
        message: t('toast.noteDeleted'),
        refresh: refresh.note,
      }),
  })

  // The phrases have no month to turn: the hook is handed no grid, so the
  // pointer is never near an edge and its timer never starts.
  const noGrid = useRef<HTMLElement>(null)
  const carry = useChipDrag({
    gridRef: noGrid,
    onEdge: () => {},
    onDrop: (noteId, target) => {
      setOver(null)
      const shelfId = target?.closest<HTMLElement>('[data-shelf-drop]')?.dataset.shelfDrop
      if (shelfId === undefined) return
      // Let go over a shelf that has it already: put back, nothing written.
      if (shelfOf(shelfId, list)?.note_ids.includes(noteId) === true) return
      put.mutate({ shelfId, noteId })
    },
  })

  const add = () => {
    const typed = splitLines(draft)
    if (typed.length > 0 && !keep.isPending) keep.mutate(typed)
  }
  const placeholder =
    shelf === null
      ? t('notes.linePlaceholder')
      : t('notes.linePlaceholderOn', { shelf: shelf.name })

  return (
    <>
      <ListDetail
        list={
          <Shelves
            shelves={shelves}
            scope={current}
            onScope={(next) => {
              setScope(next)
              setShown(PAGE)
            }}
            counts={counts}
            carrying={carry.dragging !== null}
            over={over}
            onOver={setOver}
          />
        }
        detail={
          <Pane
            label={shelf?.name ?? t('notes.lines')}
            bodyClassName="p-1.5"
            head={
              // One line and Enter, or a list pasted by lines: Shift+Enter
              // starts a line of the list by hand.
              <Textarea
                autoResize
                maxRows={6}
                rows={1}
                value={draft}
                onChange={(event) => setDraft(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === 'Enter' && !event.shiftKey) {
                    event.preventDefault()
                    add()
                  }
                }}
                placeholder={placeholder}
                aria-label={placeholder}
                className="min-w-48 flex-1"
              />
            }
            foot={
              visible.length > shown ? (
                <>
                  <span className="text-xs text-faint">
                    {t('notes.linesShown', { shown, total: visible.length })}
                  </span>
                  <Button size="xs" variant="ghost" onClick={() => setShown(shown + PAGE)}>
                    {t('notes.showMore')}
                  </Button>
                </>
              ) : undefined
            }
          >
            <Loaded
              query={lines}
              skeleton={<SkeletonList rows={8} />}
              isEmpty={() => visible.length === 0}
              emptyState={
                <EmptyState
                  plain
                  variant={filtered ? 'filtered' : 'empty'}
                  title={
                    filtered
                      ? t('notes.noMatches')
                      : shelf !== null
                        ? t('notes.shelfEmpty')
                        : current === NO_SHELF && rows.length > 0
                          ? t('notes.looseEmpty')
                          : t('notes.linesEmpty')
                  }
                  body={
                    filtered || (current === NO_SHELF && rows.length > 0)
                      ? undefined
                      : shelf !== null
                        ? t('notes.shelfEmptyBody')
                        : t('notes.linesEmptyBody')
                  }
                  className="p-2"
                />
              }
              plain
            >
              {() => (
                <ul className="flex flex-col">
                  {visible.slice(0, shown).map((line) => (
                    <LineItem
                      key={line.id}
                      line={line}
                      shelf={shelf}
                      shelves={list}
                      work={line.work_id === null ? null : (titles.get(line.work_id) ?? '…')}
                      carrying={carry.dragging?.id === line.id}
                      onGrab={(event) => {
                        // A carry let go with Escape left no drop to clear the
                        // shelf it was over; this one starts over nothing.
                        setOver(null)
                        carry.begin(event, line.id)
                      }}
                      onPatch={(change) => patch.mutate({ id: line.id, change })}
                      onAttach={() => setAttaching(line)}
                      onPromote={() => setPromoting(line)}
                      onPut={(shelfId) => put.mutate({ shelfId, noteId: line.id })}
                      onTakeOff={() => {
                        if (shelf !== null) takeOff.mutate({ shelfId: shelf.id, noteId: line.id })
                      }}
                      onDelete={() => remove.mutate(line.id)}
                    />
                  ))}
                </ul>
              )}
            </Loaded>
          </Pane>
        }
      />
      {carry.dragging !== null && (
        <CarriedLine dragging={carry.dragging} line={bodyOf(carry.dragging.id)} />
      )}
      <PickWorkDialog
        open={attaching !== null}
        onOpenChange={(open) => {
          if (!open) setAttaching(null)
        }}
        title={t('notes.toWorkTitle')}
        onPick={(picked) => {
          if (attaching === null) return
          // Kept for the work, not yet spent by it: where it stands is its
          // own word, said from the menu.
          patch.mutate(
            { id: attaching.id, change: { work_id: picked.work_id } },
            { onSuccess: () => say.ok(t('notes.attached', { title: picked.title })) },
          )
        }}
      />
      {promoting !== null && (
        <PromoteNoteDialog
          open
          onOpenChange={(open) => {
            if (!open) setPromoting(null)
          }}
          note={promoting}
          onPromoted={() => setPromoting(null)}
        />
      )}
    </>
  )
}

/**
 * The phrase that follows the pointer to a shelf: the words themselves on the
 * panel's ground, fixed to the viewport so no pane's overflow clips it on the
 * way from the right to the left, and deaf to the pointer, so what it is let
 * go over is the shelf under it rather than itself.
 */
function CarriedLine({ dragging, line }: { dragging: Dragging; line: string }) {
  return createPortal(
    <div
      aria-hidden
      className="pointer-events-none fixed max-w-80 truncate rounded-md border border-line bg-raise px-2 py-1 text-sm font-medium shadow-float [z-index:var(--z-overlay)]"
      style={{ left: dragging.ghost.left, top: dragging.ghost.top }}
    >
      {line}
    </div>,
    document.body,
  )
}
