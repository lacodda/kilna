import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { useQueryClient } from '@tanstack/react-query'
import { ListFilter, Plus, X } from 'lucide-react'
import { deleteCollection, updateCollection } from '@/lib/api/collections'
import type { Collection, CollectionPatch, ScoredWork } from '@/lib/api/types'
import { saveFilter } from '@/lib/catalogue'
import { moveTo, progressOf, standingOf } from '@/lib/collections'
import { coverImageFor } from '@/lib/cover'
import { formatTotal } from '@/lib/format'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { stageAt } from '@/lib/stages'
import { announceDeleted } from '@/lib/trash'
import { useCovers } from '@/lib/useCovers'
import { labelOf, say as sayLabel, useProfile, vocabularyOf } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Chip } from '@/components/ui/chip'
import { EmptyState } from '@/components/ui/empty-state'
import { InlineField, numberCodec } from '@/components/ui/inline-field'
import { Progress } from '@/components/ui/progress'
import { ReorderGrip, ReorderIndicator, useReorder } from '@/components/ui/reorderable-list'
import { DatePicker } from '@/components/DatePicker'
import { Pane } from '@/components/frame'
import { PickWorkDialog } from '@/components/PickWorkDialog'
import { RowMenu } from '@/components/RowMenu'
import { Select } from '@/components/AppSelect'
import { StageDial } from '@/components/StageDial'
import { GoalLine } from './GoalLine'
import { useCollectionGestures } from './useCollectionGestures'

interface Props {
  collection: Collection
  /** The catalogue's rows: the works it holds, with their verdicts. */
  rows: readonly ScoredWork[]
  today: string
  onClose: () => void
}

/**
 * An open collection, beside the grid: what it is, where it stands against
 * its goal and by its works' scores, and the works themselves in their order.
 *
 * The order is the collection's whole point - the running order of an album,
 * the chapters of a book - so the list is put in order by a grip, the way the
 * line puts any list in order, and Alt with an arrow on the keyboard. Every
 * change to it is one write of the whole list and one undo.
 */
export function CollectionDetail({ collection, rows, today, onClose }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const client = useQueryClient()
  const { config } = useProfile()
  const covers = useCovers()
  const { add, arrange } = useCollectionGestures()
  const [picking, setPicking] = useState(false)

  // The order a drop asked for, until the answer comes back with it: the
  // list must not jump back to where the row was for the length of a round
  // trip. Held with the order it was made from, so a list that changed under
  // it - an undo, another window - wins rather than being painted over.
  const [asked, setAsked] = useState<{ from: string; order: string[] } | null>(null)
  const basis = collection.work_ids.join(' ')
  // Let go of once the stored list has moved on: an undo that brings back the
  // very list the drop was made from must show it, not the drop again.
  if (asked !== null && asked.from !== basis) setAsked(null)
  const order = asked !== null && asked.from === basis ? asked.order : collection.work_ids

  const progress = progressOf(collection, today)
  const standing = standingOf({ ...collection, work_ids: order }, rows)
  const byId = new Map(standing.works.map((row) => [row.work_id, row]))

  const edit = useAppMutation({
    mutationFn: (patch: CollectionPatch) => updateCollection(collection.id, patch),
    failure: 'toast.collectionSaveFailed',
    refresh: refresh.collection,
  })

  const remove = useAppMutation({
    mutationFn: () => deleteCollection(collection.id),
    failure: 'toast.collectionSaveFailed',
    onSuccess: (deletionId) => {
      onClose()
      announceDeleted({
        client,
        deletionId,
        message: t('collections.deleted', { title: collection.title }),
        refresh: refresh.collection,
      })
    },
  })

  const put = (next: string[], message: string) => {
    setAsked({ from: basis, order: next })
    // Refused, the list goes back to what is stored rather than keeping an
    // order nobody saved.
    arrange.mutate({ collection, workIds: next, message }, { onError: () => setAsked(null) })
  }

  const reorder = useReorder({
    order,
    onMove: (id, to) => put(moveTo(order, id, to), t('collections.reordered')),
    disabled: arrange.isPending,
  })

  const showInCatalogue = () => {
    // The catalogue reads its filter from the session as it opens: the line
    // it shows is then `collection:"вЂ¦"`, and the person can widen it from
    // there like any other.
    saveFilter({ collection: collection.id })
    void navigate('/catalogue')
  }

  const kinds = config.collection_kinds

  return (
    <Pane
      label={collection.title}
      bodyClassName="flex flex-col gap-4 p-3"
      head={
        <>
          <InlineField
            label={t('collections.name')}
            labelHidden
            value={collection.title}
            // A name cannot be emptied: a collection called nothing is one
            // nobody can find again.
            onCommit={(title) => {
              if (title === null || title.trim() === '') return false
              if (title.trim() !== collection.title) edit.mutate({ title: title.trim() })
            }}
            className="min-w-0 flex-1 [&_input]:text-base [&_input]:font-semibold"
          />
          {kinds.length > 1 && (
            <Select
              aria-label={t('collections.kind')}
              value={collection.kind}
              onChange={(kind) => {
                if (kind !== '' && kind !== collection.kind) edit.mutate({ kind })
              }}
              options={kinds.map((entry) => ({ value: entry.key, label: sayLabel(entry.label) }))}
              className="w-36"
            />
          )}
          <RowMenu
            label={collection.title}
            actions={[
              {
                key: 'catalogue',
                label: t('collections.showInCatalogue'),
                onSelect: showInCatalogue,
              },
              {
                key: 'delete',
                label: t('collections.delete'),
                danger: true,
                onSelect: () => remove.mutate(),
              },
            ]}
          />
        </>
      }
      foot={
        <>
          <Button size="sm" onClick={() => setPicking(true)} disabled={add.isPending}>
            <Plus aria-hidden />
            {t('collections.addWork')}
          </Button>
          <Button size="sm" variant="ghost" onClick={showInCatalogue}>
            <ListFilter aria-hidden />
            {t('collections.showInCatalogue')}
          </Button>
          <span className="ml-auto font-mono text-xs text-faint">
            {t('collections.held', { count: progress.held })}
          </span>
        </>
      }
    >
      {/* The goal and the verdict, side by side: how far the whole is, and
          how good what it already holds is. */}
      <div className="grid grid-cols-[repeat(auto-fit,minmax(15rem,1fr))] gap-3">
        <section className="flex flex-col gap-2 rounded-lg border border-line p-3">
          <h3 className="caption">{t('collections.goalTitle')}</h3>
          <div className="flex flex-wrap items-end gap-3">
            <InlineField
              label={t('collections.target')}
              value={collection.target_size}
              codec={numberCodec}
              placeholder={t('collections.targetNone')}
              onCommit={(size) => {
                if (size !== null && (!Number.isInteger(size) || size < 1)) return false
                if (size !== collection.target_size) edit.mutate({ target_size: size })
              }}
              className="w-28"
            />
            <div className="flex min-w-0 flex-col gap-0.5">
              <span className="caption">{t('collections.due')}</span>
              <div className="flex items-center gap-1">
                <DatePicker
                  value={collection.due_on ?? ''}
                  onChange={(day) => {
                    if (day !== '' && day !== collection.due_on) edit.mutate({ due_on: day })
                  }}
                  placeholder={t('collections.dueNone')}
                  aria-label={t('collections.due')}
                  className="w-36"
                />
                {collection.due_on !== null && (
                  <Button
                    variant="icon"
                    size="icon-sm"
                    title={t('collections.dueClear')}
                    aria-label={t('collections.dueClear')}
                    onClick={() => edit.mutate({ due_on: null })}
                  >
                    <X aria-hidden />
                  </Button>
                )}
              </div>
            </div>
          </div>
          {progress.fraction !== null && (
            <Progress
              size="sm"
              tone={progress.met ? 'good' : 'warn'}
              value={progress.fraction * 100}
              label={t('collections.progressLabel', { title: collection.title })}
            />
          )}
          <GoalLine progress={progress} className="text-sm text-dim" />
        </section>

        <section className="flex flex-col gap-2 rounded-lg border border-line p-3">
          <h3 className="caption">{t('collections.verdictTitle')}</h3>
          {standing.average === null ? (
            <p className="text-sm text-faint">{t('collections.noVerdict')}</p>
          ) : (
            <>
              <p className="flex items-baseline gap-2">
                <span className="font-mono text-2xl font-semibold tabular-nums">
                  {formatTotal(standing.average)}
                </span>
                <span className="text-sm text-dim">
                  {t('collections.averageOf', {
                    count: standing.scored,
                    total: standing.works.length,
                  })}
                </span>
              </p>
              {standing.weakest !== null && (
                <Verdict
                  tone="warn"
                  word={t('collections.weakestWord')}
                  row={standing.weakest}
                  onOpen={() => void navigate(`/works/${standing.weakest!.work_id}`)}
                />
              )}
              {standing.strongest !== null && (
                <Verdict
                  tone="good"
                  word={t('collections.strongestWord')}
                  row={standing.strongest}
                  onOpen={() => void navigate(`/works/${standing.strongest!.work_id}`)}
                />
              )}
            </>
          )}
        </section>
      </div>

      <InlineField
        label={t('collections.description')}
        value={collection.description}
        placeholder={t('collections.descriptionNone')}
        onCommit={(text) => {
          const next = text === null || text.trim() === '' ? null : text.trim()
          if (next !== collection.description) edit.mutate({ description: next })
        }}
      />

      <section className="flex flex-col gap-1.5">
        <h3 className="caption">{t('collections.works')}</h3>
        {order.length === 0 ? (
          <EmptyState
            plain
            title={t('collections.emptyTitle')}
            body={t('collections.emptyBody')}
            className="p-2"
          />
        ) : (
          // The line where a row would land is drawn against this box.
          <div {...reorder.listProps} className="relative">
            <ol className="flex flex-col gap-0.5">
              {order.map((workId, index) => {
                const row = byId.get(workId)
                const vocabulary = vocabularyOf(config, row?.kind)
                const stop = stageAt(config, row?.stage ?? null)
                return (
                  <li
                    key={workId}
                    {...reorder.rowProps(workId)}
                    tabIndex={0}
                    aria-label={row?.title ?? workId}
                    className={cn(
                      'flex min-w-0 items-center gap-2 rounded-md px-1.5 py-1 text-sm hover:bg-soft focus-visible:outline-2 focus-visible:outline-accent',
                      reorder.dragging === workId && 'opacity-50',
                    )}
                  >
                    {order.length > 1 ? (
                      <ReorderGrip {...reorder.gripProps(workId)} title={t('collections.move')} />
                    ) : (
                      <span aria-hidden className="w-3.5 shrink-0" />
                    )}
                    <span className="w-6 shrink-0 text-right font-mono text-xs text-faint tabular-nums">
                      {index + 1}
                    </span>
                    <span
                      aria-hidden
                      className="size-5 shrink-0 rounded-sm"
                      style={{ background: coverImageFor(workId, covers.get(workId)) }}
                    />
                    <Button
                      variant="link"
                      className="min-w-0 flex-1 justify-start truncate text-left text-sm text-text"
                      onClick={() => void navigate(`/works/${workId}`)}
                    >
                      {row?.title ?? t('collections.unknownWork')}
                    </Button>
                    {row !== undefined && (
                      <Chip variant="soft" className="shrink-0">
                        {labelOf(config.work_kinds, row.kind)}
                      </Chip>
                    )}
                    <StageDial percent={row?.stage ?? null} stage={stop} size={14} />
                    <span className="w-24 shrink-0 text-right font-mono text-xs text-dim tabular-nums">
                      {row?.total === null || row?.total === undefined
                        ? t('collections.unjudgedOne')
                        : `${row.tier === null ? '' : `${labelOf(vocabulary.tiers, row.tier)} В· `}${formatTotal(row.total)}`}
                    </span>
                    <Button
                      variant="icon"
                      size="icon-sm"
                      title={t('collections.takeOut')}
                      aria-label={t('collections.takeOutWork', { title: row?.title ?? workId })}
                      disabled={arrange.isPending}
                      onClick={() =>
                        put(
                          order.filter((id) => id !== workId),
                          t('collections.takenOut', { title: row?.title ?? '' }),
                        )
                      }
                    >
                      <X aria-hidden />
                    </Button>
                  </li>
                )
              })}
            </ol>
            <ReorderIndicator offset={reorder.slotOffset} />
          </div>
        )}
      </section>

      <PickWorkDialog
        open={picking}
        onOpenChange={setPicking}
        title={t('collections.addTo', { title: collection.title })}
        onPick={(work) => add.mutate({ collection, workIds: [work.work_id] })}
      />
    </Pane>
  )
}

/** The weakest or the strongest work, named, with its total, a click from its card. */
function Verdict({
  tone,
  word,
  row,
  onOpen,
}: {
  tone: 'good' | 'warn'
  word: string
  row: ScoredWork
  onOpen: () => void
}) {
  return (
    <p className="flex min-w-0 items-center gap-2 text-sm">
      <Chip variant={tone} className="shrink-0">
        {word}
      </Chip>
      <Button variant="link" className="min-w-0 truncate text-sm" onClick={onOpen}>
        {row.title}
      </Button>
      <span className="ml-auto shrink-0 font-mono text-xs text-dim tabular-nums">
        {row.total === null ? '' : formatTotal(row.total)}
      </span>
    </p>
  )
}
