import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { GripVertical, Pin, PinOff, X } from 'lucide-react'
import { deleteFocusNote, reorderFocusNotes, updateFocusNote } from '@/lib/api/focus'
import type { FocusNote } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
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
    <Widget title={t('focus.title')}>
      <Loaded query={notes} plain skeleton={<Skeleton className="h-4 w-full" />}>
        {(lines) => <NoteList notes={lines} onSelect={onSelect} />}
      </Loaded>
      <AddNote />
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

  if (notes.length === 0) return null

  /** Drop `dragging` where `target` sits, and send the whole arrangement. */
  const drop = (target: string) => {
    if (dragging === null || dragging === target) return

    const order = notes.map((note) => note.id).filter((id) => id !== dragging)
    order.splice(order.indexOf(target), 0, dragging)
    move.mutate(order)
  }

  return (
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
  )
}
