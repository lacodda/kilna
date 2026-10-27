import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { GripVertical, MoreHorizontal, Pin, PinOff, X } from 'lucide-react'
import { deleteFocusNote, reorderFocusNotes, updateFocusNote } from '@/lib/api/focus'
import type { FocusNote, FocusNotePatch } from '@/lib/api/types'
import { formatDay } from '@/lib/format'
import { today } from '@/lib/month'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Menu, MenuItem, MenuPopup, MenuSeparator, MenuTrigger } from '@/components/ui/menu'
import { PickWorkDialog } from '@/components/PickWorkDialog'
import { Skeleton } from '@/components/ui/skeleton'
import { Loaded } from '@/components/Loaded'
import { AddNote } from '@/features/dashboard/AddNote'
import { Widget } from '@/features/dashboard/Widget'

interface Props {
  onSelect: (workId: string, tab?: string) => void
}

/**
 * The lines the person put on the dashboard themselves.
 *
 * The other half of what was the focus board, and the half that is theirs: a
 * finding appears when its complaint becomes true and leaves when it stops
 * being true; a line stays until they rub it out. That difference is why only
 * a line has a pin and an order - keeping a finding at the top would promise
 * to hold something that is about to vanish on its own.
 *
 * In the side column, under what the workspace noticed and above what
 * happened since: read with the decisions, kept out of their way.
 */
export function NotesWidget({ onSelect }: Props) {
  const { t } = useTranslation()
  const notes = useQuery(queries.focusNotes())

  return (
    // The field stands under the lines: a long list scrolls, the way to add
    // one more does not go with it.
    <Widget title={t('focus.title')} scroll={t('focus.title')} foot={<AddNote />}>
      <Loaded query={notes} plain skeleton={<Skeleton className="h-4 w-full" />}>
        {(lines) => <NoteList notes={lines} onSelect={onSelect} />}
      </Loaded>
    </Widget>
  )
}

/** The person's own lines, in the order they arranged them. */
function NoteList({ notes, onSelect }: { notes: readonly FocusNote[] } & Props) {
  const { t } = useTranslation()
  const [dragging, setDragging] = useState<string | null>(null)
  const [over, setOver] = useState<string | null>(null)

  const pin = useAppMutation({
    mutationFn: ({ id, pinned }: { id: string; pinned: boolean }) =>
      updateFocusNote(id, { pinned }),
    refresh: [keys.focusNotes],
  })

  const rub = useAppMutation({
    mutationFn: deleteFocusNote,
    refresh: [keys.focusNotes],
  })

  const move = useAppMutation({
    mutationFn: reorderFocusNotes,
    refresh: [keys.focusNotes],
  })

  // A line's day and its work (v0.105): set from its menu, read on the
  // line, and a line whose day has come stands under "Today" at the top.
  const edit = useAppMutation({
    mutationFn: ({ id, patch }: { id: string; patch: FocusNotePatch }) =>
      updateFocusNote(id, patch),
    refresh: [keys.focusNotes, keys.focus],
  })
  const [tying, setTying] = useState<string | null>(null)

  if (notes.length === 0) return null

  const day = today()
  const due = notes
    .filter((note) => note.due_on !== null && note.due_on <= day)
    .sort((a, b) => (a.due_on ?? '').localeCompare(b.due_on ?? ''))
    .slice(0, TODAY_MOST)
  const open = (note: FocusNote) => {
    if (note.work_id !== null) onSelect(note.work_id)
  }

  /** Drop `dragging` where `target` sits, and send the whole arrangement. */
  const drop = (target: string) => {
    if (dragging === null || dragging === target) return

    const order = notes.map((note) => note.id).filter((id) => id !== dragging)
    order.splice(order.indexOf(target), 0, dragging)
    move.mutate(order)
  }

  return (
    <>
      {due.length > 0 && (
        <section
          aria-label={t('focus.today')}
          className="mb-2 flex flex-col gap-0.5 rounded-md bg-accent-soft px-2 py-1.5"
        >
          <p className="text-xs font-semibold text-accent">{t('focus.today')}</p>
          <ul className="flex flex-col">
            {due.map((note) => (
              <li key={note.id} className="flex min-w-0 items-baseline gap-2 text-sm">
                {note.work_id === null ? (
                  <span className="min-w-0 flex-1 truncate">{note.body}</span>
                ) : (
                  <Button
                    variant="link"
                    className="min-w-0 flex-1 justify-start truncate text-left"
                    onClick={() => open(note)}
                  >
                    {note.body}
                  </Button>
                )}
                <DueMark due={note.due_on as string} day={day} />
              </li>
            ))}
          </ul>
        </section>
      )}
      <ul className="flex flex-col">
        {notes.map((note) => (
          <li
            key={note.id}
            onDragOver={(event) => {
              event.preventDefault()
              setOver(note.id)
            }}
            onDragLeave={() => setOver((current) => (current === note.id ? null : current))}
            onDrop={(event) => {
              event.preventDefault()
              drop(note.id)
              setOver(null)
            }}
            className={cn(
              'flex min-w-0 items-center gap-2 rounded-sm border-b border-line py-1 text-sm last:border-b-0',
              dragging === note.id && 'opacity-40',
              // Where the line will land, tinted rather than outlined: a row
              // with a hairline under it has no border of its own to light up.
              over === note.id && dragging !== note.id && 'bg-accent-soft',
            )}
          >
            {/* The handle, and only the handle, is draggable — the same rule the
              calendar follows, for the same reason: a whole draggable row puts
              every click in a race with a drag. */}
            <span
              draggable
              onDragStart={(event) => {
                event.dataTransfer.setData('text/plain', note.id)
                event.dataTransfer.effectAllowed = 'move'
                setDragging(note.id)
              }}
              onDragEnd={() => {
                setDragging(null)
                setOver(null)
              }}
              title={t('focus.dragHandle')}
              className="shrink-0 cursor-grab text-faint hover:text-dim"
            >
              <GripVertical aria-hidden className="size-3.5" />
            </span>

            {/* A line tied to a work is the way to it, so it reads as a link;
              one about nothing in particular is only words. */}
            {note.work_id === null ? (
              <span className="min-w-0 flex-1 leading-relaxed">{note.body}</span>
            ) : (
              <Button
                variant="link"
                onClick={() => onSelect(note.work_id as string)}
                className="min-w-0 flex-1 justify-start text-left leading-relaxed whitespace-normal"
              >
                {note.body}
              </Button>
            )}

            {note.due_on !== null && <DueMark due={note.due_on} day={day} />}

            <Menu>
              <MenuTrigger
                render={
                  <Button
                    variant="icon"
                    size="icon-xs"
                    aria-label={t('focus.more')}
                    title={t('focus.more')}
                  />
                }
              >
                <MoreHorizontal aria-hidden />
              </MenuTrigger>
              <MenuPopup align="end">
                <MenuItem onClick={() => edit.mutate({ id: note.id, patch: { due_on: day } })}>
                  {t('focus.dueToday')}
                </MenuItem>
                <MenuItem
                  onClick={() => edit.mutate({ id: note.id, patch: { due_on: plusDays(day, 1) } })}
                >
                  {t('focus.dueTomorrow')}
                </MenuItem>
                <MenuItem
                  onClick={() => edit.mutate({ id: note.id, patch: { due_on: plusDays(day, 7) } })}
                >
                  {t('focus.dueWeek')}
                </MenuItem>
                {note.due_on !== null && (
                  <MenuItem onClick={() => edit.mutate({ id: note.id, patch: { due_on: null } })}>
                    {t('focus.dueClear')}
                  </MenuItem>
                )}
                <MenuSeparator />
                <MenuItem onClick={() => setTying(note.id)}>{t('focus.tie')}</MenuItem>
                {note.work_id !== null && (
                  <MenuItem onClick={() => edit.mutate({ id: note.id, patch: { work_id: null } })}>
                    {t('focus.untie')}
                  </MenuItem>
                )}
              </MenuPopup>
            </Menu>

            <Button
              variant="icon"
              size="icon-xs"
              aria-label={note.pinned_at === null ? t('focus.pin') : t('focus.unpin')}
              title={note.pinned_at === null ? t('focus.pin') : t('focus.unpin')}
              onClick={() => pin.mutate({ id: note.id, pinned: note.pinned_at === null })}
            >
              {/* The accent is the glyph's, not the button's: it is what says
                this line is pinned, and it stays whatever the button's hover
                does. */}
              {note.pinned_at === null ? (
                <Pin aria-hidden />
              ) : (
                <PinOff aria-hidden className="text-accent" />
              )}
            </Button>

            <Button
              variant="icon"
              size="icon-xs"
              aria-label={t('focus.remove')}
              title={t('focus.remove')}
              onClick={() => rub.mutate(note.id)}
            >
              <X aria-hidden />
            </Button>
          </li>
        ))}
      </ul>
      <PickWorkDialog
        open={tying !== null}
        onOpenChange={(open) => {
          if (!open) setTying(null)
        }}
        title={t('focus.tieTitle')}
        onPick={(work) => {
          if (tying !== null) edit.mutate({ id: tying, patch: { work_id: work.work_id } })
          setTying(null)
        }}
      />
    </>
  )
}

/** How many lines "Today" holds: a day's plan is a few things, not a list. */
const TODAY_MOST = 3

/** A line's day: faint while it is ahead, the warning colour once it came. */
function DueMark({ due, day }: { due: string; day: string }) {
  const { t } = useTranslation()
  return (
    <span
      className={cn('shrink-0 font-mono text-xs', due <= day ? 'text-warn' : 'text-faint')}
      title={t('focus.dueOn', { day: formatDay(due) })}
    >
      {formatDay(due)}
    </span>
  )
}

/** The ISO date `days` after `day`. */
function plusDays(day: string, days: number): string {
  const [year, month, date] = day.split('-').map(Number) as [number, number, number]
  return new Date(Date.UTC(year, month - 1, date + days)).toISOString().slice(0, 10)
}
