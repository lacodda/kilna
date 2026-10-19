import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQueryClient, type UseQueryResult } from '@tanstack/react-query'
import { createShelf, deleteShelf, moveShelf, renameShelf } from '@/lib/api/notes'
import type { ShelfView } from '@/lib/api/types'
import { ALL_LINES, NO_SHELF, type LineScope } from '@/lib/lines'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { announceDeleted } from '@/lib/trash'
import { cn } from '@/lib/utils'
import { Divider } from '@/components/ui/divider'
import { EmptyState } from '@/components/ui/empty-state'
import { Input } from '@/components/ui/input'
import { RowButton } from '@/components/ui/list-row'
import {
  ReorderGrip,
  ReorderIndicator,
  useReorder,
  type Reorder,
} from '@/components/ui/reorderable-list'
import { SkeletonList } from '@/components/ui/skeleton'
import { Pane } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { RowMenu, type RowAction } from '@/components/RowMenu'

interface Props {
  shelves: UseQueryResult<ShelfView[]>
  scope: LineScope
  onScope: (scope: LineScope) => void
  /** How many phrases the filters leave, and how many of them are on no shelf. */
  counts: { all: number; loose: number }
  /** A phrase is being carried: the shelves are where it may land. */
  carrying: boolean
  /** The shelf the carried phrase is over. */
  over: string | null
  onOver: (shelfId: string | null) => void
}

/**
 * The shelves of phrases, in the owner's order (ADR 0064), under the two
 * parts that are not shelves: every phrase, and the phrases on no shelf yet.
 *
 * A shelf is named in the field at the foot and made with Enter, renamed in
 * place, put elsewhere in the order by its grip - or from its menu, which is
 * the way a keyboard has besides Alt with an arrow - and deleted from the
 * menu into the trash, its phrases staying. Each shelf is also where a phrase
 * carried from the right lands: it says so under the pointer.
 */
export function Shelves({ shelves, scope, onScope, counts, carrying, over, onOver }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const [name, setName] = useState('')
  const list = shelves.data ?? []

  const create = useAppMutation({
    mutationFn: (named: string) => createShelf(named),
    failure: 'shelves.failed',
    refresh: refresh.note,
    // The shelf just made is opened: phrases typed next go onto it.
    onSuccess: (made) => {
      setName('')
      onScope(made.id)
    },
  })
  const rename = useAppMutation({
    mutationFn: ({ id, named }: { id: string; named: string }) => renameShelf(id, named),
    failure: 'shelves.failed',
    refresh: refresh.note,
  })
  const move = useAppMutation({
    mutationFn: ({ id, index }: { id: string; index: number }) => moveShelf(id, index),
    failure: 'shelves.failed',
    refresh: refresh.note,
  })
  const remove = useAppMutation({
    mutationFn: (shelf: ShelfView) => deleteShelf(shelf.id),
    failure: 'shelves.failed',
    onSuccess: (deletionId, shelf) =>
      announceDeleted({
        client,
        deletionId,
        message: t('shelves.deleted', { name: shelf.name }),
        refresh: refresh.note,
      }),
  })

  // Nothing moves until the grip is let go: one write per drop, not per row
  // crossed on the way.
  const reorder = useReorder({
    order: list.map((shelf) => shelf.id),
    onMove: (id, index) => move.mutate({ id, index }),
    disabled: move.isPending,
  })

  return (
    <Pane
      label={t('shelves.title')}
      bodyClassName="flex flex-col gap-0.5 p-1.5"
      foot={
        <Input
          value={name}
          onChange={(event) => setName(event.target.value)}
          onKeyDown={(event) => {
            const named = name.trim()
            if (event.key === 'Enter' && named !== '' && !create.isPending) create.mutate(named)
          }}
          placeholder={t('shelves.new')}
          aria-label={t('shelves.new')}
          className="flex-1"
        />
      }
    >
      <RowButton selected={scope === ALL_LINES} onClick={() => onScope(ALL_LINES)} end={counts.all}>
        {t('shelves.all')}
      </RowButton>
      <RowButton selected={scope === NO_SHELF} onClick={() => onScope(NO_SHELF)} end={counts.loose}>
        {t('shelves.none')}
      </RowButton>
      <Divider spacing="sm" label={t('shelves.title')} decorative />
      <Loaded
        query={shelves}
        skeleton={<SkeletonList rows={3} />}
        isEmpty={(data) => data.length === 0}
        emptyState={
          <EmptyState
            plain
            title={t('shelves.empty')}
            body={t('shelves.emptyBody')}
            className="p-2"
          />
        }
        plain
      >
        {() => (
          // The line where a dragged shelf would land is drawn against this
          // box, so it holds the list rather than standing inside it.
          <div {...reorder.listProps} className="relative">
            <ul className="flex flex-col gap-0.5">
              {list.map((shelf, index) => (
                <ShelfRow
                  key={shelf.id}
                  shelf={shelf}
                  selected={scope === shelf.id}
                  over={carrying && over === shelf.id}
                  dragging={reorder.dragging === shelf.id}
                  rowProps={reorder.rowProps(shelf.id)}
                  grip={list.length > 1 ? reorder.gripProps(shelf.id) : null}
                  onOpen={() => onScope(shelf.id)}
                  onPointerEnter={() => {
                    if (carrying) onOver(shelf.id)
                  }}
                  onPointerLeave={() => {
                    if (over === shelf.id) onOver(null)
                  }}
                  onRename={(named) => rename.mutate({ id: shelf.id, named })}
                  actions={[
                    ...(index > 0
                      ? [
                          {
                            key: 'up',
                            label: t('shelves.moveUp'),
                            onSelect: () => move.mutate({ id: shelf.id, index: index - 1 }),
                          },
                        ]
                      : []),
                    ...(index < list.length - 1
                      ? [
                          {
                            key: 'down',
                            label: t('shelves.moveDown'),
                            onSelect: () => move.mutate({ id: shelf.id, index: index + 1 }),
                          },
                        ]
                      : []),
                    {
                      key: 'delete',
                      label: t('shelves.delete'),
                      onSelect: () => remove.mutate(shelf),
                      danger: true,
                    },
                  ]}
                />
              ))}
            </ul>
            <ReorderIndicator offset={reorder.slotOffset} />
          </div>
        )}
      </Loaded>
    </Pane>
  )
}

interface RowProps {
  shelf: ShelfView
  selected: boolean
  /** A carried phrase is over it, and would land here. */
  over: boolean
  /** It is the shelf being put elsewhere in the order. */
  dragging: boolean
  rowProps: ReturnType<Reorder<string>['rowProps']>
  /** The grip's handlers, or none when there is nowhere to move to. */
  grip: ReturnType<Reorder<string>['gripProps']> | null
  onOpen: () => void
  onPointerEnter: () => void
  onPointerLeave: () => void
  onRename: (name: string) => void
  /** The menu's acts besides renaming, which the row does itself. */
  actions: RowAction[]
}

/** One shelf: its grip, its name and how many phrases it holds, its menu. */
function ShelfRow({
  shelf,
  selected,
  over,
  dragging,
  rowProps,
  grip,
  onOpen,
  onPointerEnter,
  onPointerLeave,
  onRename,
  actions,
}: RowProps) {
  const { t } = useTranslation()
  const [editing, setEditing] = useState(false)
  const [text, setText] = useState(shelf.name)

  const save = () => {
    setEditing(false)
    const next = text.trim()
    if (next !== '' && next !== shelf.name) onRename(next)
    else setText(shelf.name)
  }

  return (
    <li
      {...rowProps}
      // Where a carried phrase is let go is read off the element under the
      // pointer, by this attribute (`LineBank`).
      data-shelf-drop={shelf.id}
      onPointerEnter={onPointerEnter}
      onPointerLeave={onPointerLeave}
      className={cn(
        'flex items-center gap-1 rounded-md',
        dragging && 'opacity-50',
        over && 'bg-accent-soft outline-2 -outline-offset-2 outline-accent outline-dashed',
      )}
    >
      {grip === null ? (
        <span aria-hidden className="w-3.5 shrink-0" />
      ) : (
        <ReorderGrip {...grip} title={t('shelves.move')} />
      )}
      {editing ? (
        <Input
          autoFocus
          value={text}
          onChange={(event) => setText(event.target.value)}
          onBlur={save}
          onKeyDown={(event) => {
            if (event.key === 'Enter') event.currentTarget.blur()
            if (event.key === 'Escape') {
              setText(shelf.name)
              setEditing(false)
            }
          }}
          aria-label={t('shelves.name')}
          className="min-w-0 flex-1"
        />
      ) : (
        <RowButton
          selected={selected}
          onClick={onOpen}
          // The name is renamed where it stands: a double-click, or the menu.
          onDoubleClick={() => setEditing(true)}
          end={shelf.note_ids.length}
          className="min-w-0 flex-1"
        >
          {shelf.name}
        </RowButton>
      )}
      <RowMenu
        label={t('shelves.menu')}
        actions={[
          { key: 'rename', label: t('shelves.rename'), onSelect: () => setEditing(true) },
          ...actions,
        ]}
      />
    </li>
  )
}
