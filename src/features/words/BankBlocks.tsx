import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQueryClient, type UseQueryResult } from '@tanstack/react-query'
import { createBlock, deleteBlock, moveBlock, renameBlock } from '@/lib/api/register'
import type { BlockView } from '@/lib/api/types'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { announceDeleted } from '@/lib/trash'
import { cn } from '@/lib/utils'
import { ALL_WORDS, NO_BLOCK, type BankScope } from '@/lib/words'
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
  blocks: UseQueryResult<BlockView[]>
  scope: BankScope
  onScope: (scope: BankScope) => void
  /** How many words the whole bank holds, and how many are in no block. */
  counts: { all: number; loose: number }
  /** A word is being carried: the blocks are where it may land. */
  carrying: boolean
  /** The block the carried word is over. */
  over: string | null
  onOver: (blockId: string | null) => void
}

/**
 * The blocks of the bank, in the owner's order (ADR 0052), under the two
 * parts that are not blocks: every word, and the words in no block yet.
 *
 * A block is named in the field at the foot and made with Enter, renamed in
 * place, put elsewhere in the order by its grip - or from its menu, which is
 * the way a keyboard has besides Alt with an arrow - and deleted from the
 * menu into the trash, its words staying in the bank. Each block is also
 * where a word carried from the right lands: it says so under the pointer.
 */
export function BankBlocks({ blocks, scope, onScope, counts, carrying, over, onOver }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const [name, setName] = useState('')
  const list = blocks.data ?? []

  const create = useAppMutation({
    mutationFn: (named: string) => createBlock(named),
    failure: 'words.blockFailed',
    refresh: refresh.term,
    // The block just made is opened: words typed next go into it.
    onSuccess: (made) => {
      setName('')
      onScope(made.id)
    },
  })
  const rename = useAppMutation({
    mutationFn: ({ id, named }: { id: string; named: string }) => renameBlock(id, named),
    failure: 'words.blockFailed',
    refresh: refresh.term,
  })
  const move = useAppMutation({
    mutationFn: ({ id, index }: { id: string; index: number }) => moveBlock(id, index),
    failure: 'words.blockFailed',
    refresh: refresh.term,
  })
  const remove = useAppMutation({
    mutationFn: (block: BlockView) => deleteBlock(block.id),
    failure: 'words.blockFailed',
    onSuccess: (deletionId, block) =>
      announceDeleted({
        client,
        deletionId,
        message: t('words.blockDeleted', { name: block.name }),
        refresh: refresh.term,
      }),
  })

  // Nothing moves until the grip is let go: one write per drop, not per row
  // crossed on the way.
  const reorder = useReorder({
    order: list.map((block) => block.id),
    onMove: (id, index) => move.mutate({ id, index }),
    disabled: move.isPending,
  })

  return (
    <Pane
      label={t('words.blocks')}
      bodyClassName="flex flex-col gap-0.5 p-1.5"
      foot={
        <Input
          value={name}
          onChange={(event) => setName(event.target.value)}
          onKeyDown={(event) => {
            const named = name.trim()
            if (event.key === 'Enter' && named !== '' && !create.isPending) create.mutate(named)
          }}
          placeholder={t('words.newBlock')}
          aria-label={t('words.newBlock')}
          className="flex-1"
        />
      }
    >
      <RowButton selected={scope === ALL_WORDS} onClick={() => onScope(ALL_WORDS)} end={counts.all}>
        {t('words.allWords')}
      </RowButton>
      <RowButton selected={scope === NO_BLOCK} onClick={() => onScope(NO_BLOCK)} end={counts.loose}>
        {t('words.noBlock')}
      </RowButton>
      <Divider spacing="sm" label={t('words.blocks')} decorative />
      <Loaded
        query={blocks}
        skeleton={<SkeletonList rows={3} />}
        isEmpty={(data) => data.length === 0}
        emptyState={
          <EmptyState
            plain
            title={t('words.noBlocks')}
            body={t('words.noBlocksBody')}
            className="p-2"
          />
        }
        plain
      >
        {() => (
          // The line where a dragged block would land is drawn against this
          // box, so it holds the list rather than standing inside it.
          <div {...reorder.listProps} className="relative">
            <ul className="flex flex-col gap-0.5">
              {list.map((block, index) => (
                <BlockRow
                  key={block.id}
                  block={block}
                  selected={scope === block.id}
                  over={carrying && over === block.id}
                  dragging={reorder.dragging === block.id}
                  rowProps={reorder.rowProps(block.id)}
                  grip={list.length > 1 ? reorder.gripProps(block.id) : null}
                  onOpen={() => onScope(block.id)}
                  onPointerEnter={() => {
                    if (carrying) onOver(block.id)
                  }}
                  onPointerLeave={() => {
                    if (over === block.id) onOver(null)
                  }}
                  onRename={(named) => rename.mutate({ id: block.id, named })}
                  actions={[
                    ...(index > 0
                      ? [
                          {
                            key: 'up',
                            label: t('words.moveUp'),
                            onSelect: () => move.mutate({ id: block.id, index: index - 1 }),
                          },
                        ]
                      : []),
                    ...(index < list.length - 1
                      ? [
                          {
                            key: 'down',
                            label: t('words.moveDown'),
                            onSelect: () => move.mutate({ id: block.id, index: index + 1 }),
                          },
                        ]
                      : []),
                    {
                      key: 'delete',
                      label: t('words.deleteBlock'),
                      onSelect: () => remove.mutate(block),
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
  block: BlockView
  selected: boolean
  /** A carried word is over it, and would land here. */
  over: boolean
  /** It is the block being put elsewhere in the order. */
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

/** One block: its grip, its name and how many words it holds, its menu. */
function BlockRow({
  block,
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
  const [text, setText] = useState(block.name)

  const save = () => {
    setEditing(false)
    const next = text.trim()
    if (next !== '' && next !== block.name) onRename(next)
    else setText(block.name)
  }

  return (
    <li
      {...rowProps}
      // Where a carried word is let go is read off the element under the
      // pointer, by this attribute (`WordBank`).
      data-block-drop={block.id}
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
        <ReorderGrip {...grip} title={t('words.moveBlock')} />
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
              setText(block.name)
              setEditing(false)
            }
          }}
          aria-label={t('words.blockName')}
          className="min-w-0 flex-1"
        />
      ) : (
        <RowButton
          selected={selected}
          onClick={onOpen}
          // The name is renamed where it stands: a double-click, or the menu.
          onDoubleClick={() => setEditing(true)}
          end={block.term_ids.length}
          className="min-w-0 flex-1"
        >
          {block.name}
        </RowButton>
      )}
      <RowMenu
        label={t('words.blockMenu')}
        actions={[
          { key: 'rename', label: t('words.rename'), onSelect: () => setEditing(true) },
          ...actions,
        ]}
      />
    </li>
  )
}
