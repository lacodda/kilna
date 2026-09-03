import { type ReactNode, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ArrowDown, ArrowUp } from 'lucide-react'
import { catalogue as fetchCatalogue, createWork, deleteWork, type ScoredWork } from '@/lib/api'
import {
  isNarrowed,
  loadSort,
  narrow,
  saveSort,
  sortRows,
  toggleSort,
  type CatalogueFilter,
  type Gap,
  type Sort,
  type SortColumn,
} from '@/lib/catalogue'
import { keys } from '@/lib/query'
import { announceDeleted } from '@/lib/trash'
import { say } from '@/lib/toast'
import { labelOf, useProfile } from '@/lib/useProfile'
import type { Tab } from '@/components/card/tabs'
import { BulkActions } from '@/components/assistant/BulkActions'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/EmptyState'
import { Input } from '@/components/ui/input'
import { Select } from '@/components/ui/AppSelect'
import { RowContextMenu, RowMenu, type RowAction } from '@/components/ui/RowMenu'
import { SkeletonList } from '@/components/ui/Skeleton'
import { cn } from '@/lib/utils'

interface Props {
  /** Opens a work, optionally straight onto one of its tabs. */
  onSelect: (workId: string, tab?: Tab) => void
}

const GAPS: Gap[] = ['unscored', 'unscheduled', 'stale']

/**
 * Every work there is — the only list in the app.
 *
 * It used to show what had been scored, beside a second list of everything on
 * the Works screen. Two lists of the same things is one too many, and the
 * mockup only ever had this one: adding, searching and filtering moved here in
 * v0.21 and the other screen went away.
 */
export function Catalogue({ onSelect }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const client = useQueryClient()
  const [filter, setFilter] = useState<CatalogueFilter>({})
  const [sort, setSort] = useState<Sort>(loadSort)
  const [title, setTitle] = useState('')

  const rows = useQuery({
    queryKey: keys.catalogue,
    queryFn: fetchCatalogue,
  })

  const add = useMutation({
    mutationFn: createWork,
    onSuccess: (work) => {
      setTitle('')
      void client.invalidateQueries({ queryKey: keys.works })
      void client.invalidateQueries({ queryKey: keys.catalogue })
      void client.invalidateQueries({ queryKey: keys.workspace })
      void client.invalidateQueries({ queryKey: keys.journal })
      say.ok(t('toast.workCreated'))
      // Straight into the new work: adding one is the start of writing it, not
      // an entry in a list to admire.
      onSelect(work.id)
    },
    onError: (cause) => say.failedTo(t('toast.workSaveFailed'), cause),
  })

  const submit = () => {
    const trimmed = title.trim()
    if (trimmed === '') return

    const kind = profile.config.work_kinds[0]?.key
    if (kind === undefined) return

    add.mutate({ kind, title: trimmed })
  }

  // Deliberately not remembered across a restart, unlike the sort: a selection
  // is about the click you are about to make, and finding rows still ticked
  // tomorrow is a way to act on the wrong ones.
  const [selected, setSelected] = useState<ReadonlySet<string>>(new Set())

  const remove = useMutation({
    mutationFn: async (workIds: readonly string[]) => {
      const ids: string[] = []
      for (const workId of workIds) {
        ids.push(await deleteWork(workId))
      }
      return ids
    },
    onSuccess: (deletionIds) => {
      setSelected(new Set())
      announceDeleted({
        client,
        deletionIds,
        message: t('catalogue.deleted', { count: deletionIds.length }),
        refresh: [keys.works, keys.catalogue, keys.workspace],
      })
    },
    onError: (cause) => say.failedTo(t('toast.workSaveFailed'), cause),
  })

  const reorder = (column: SortColumn) => {
    const next = toggleSort(sort, column)
    setSort(next)
    saveSort(next)
  }

  const set = (change: Partial<CatalogueFilter>) =>
    setFilter((current) => ({ ...current, ...change }))

  return (
    <div className="flex flex-col gap-4">
      <form
        className="flex gap-2"
        onSubmit={(event) => {
          event.preventDefault()
          submit()
        }}
      >
        <Input
          className="max-w-96"
          value={title}
          onChange={(event) => setTitle(event.target.value)}
          placeholder={t('works.newPlaceholder')}
          aria-label={t('works.newPlaceholder')}
        />
        <Button type="submit" variant="primary" disabled={title.trim() === '' || add.isPending}>
          {t('works.add')}
        </Button>
      </form>

      <div className="flex flex-wrap items-center gap-2">
        <Input
          className="max-w-64"
          value={filter.search ?? ''}
          onChange={(event) => set({ search: event.target.value || undefined })}
          placeholder={t('works.search')}
          aria-label={t('works.search')}
        />
        <Select
          className="w-44"
          aria-label={t('works.status')}
          value={filter.status ?? ''}
          onChange={(value) => set({ status: value || undefined })}
          placeholder={t('works.anyStatus')}
          options={profile.config.statuses.map((s) => ({ value: s.key, label: s.label }))}
        />
        <Select
          className="w-44"
          aria-label={t('works.kind')}
          value={filter.kind ?? ''}
          onChange={(value) => set({ kind: value || undefined })}
          placeholder={t('works.anyKind')}
          options={profile.config.work_kinds.map((k) => ({ value: k.key, label: k.label }))}
        />
        <Select
          className="w-44"
          aria-label={t('catalogue.tier')}
          value={filter.tier ?? ''}
          onChange={(value) => set({ tier: value || undefined })}
          placeholder={t('catalogue.anyTier')}
          options={profile.config.tiers.map((tier) => ({ value: tier.key, label: tier.label }))}
        />
      </div>

      {/* Chips carry their words, not just an icon. The predecessor tried icons
          alone and nobody could tell which filter was on. */}
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-[11px] font-medium uppercase tracking-[0.09em] text-faint">
          {t('catalogue.gaps')}
        </span>
        {GAPS.map((gap) => {
          const active = filter.gap === gap
          return (
            <button
              key={gap}
              type="button"
              // Clicking the chip that is already on turns it off: one gap at a
              // time, and no separate way to undo it.
              onClick={() => set({ gap: active ? undefined : gap })}
              aria-pressed={active}
              title={t(`catalogue.gapHint.${gap}`)}
              className={cn(
                'cursor-pointer rounded-full border px-2.5 py-0.5 text-[11.5px] transition-colors',
                active
                  ? 'border-transparent bg-accent-soft font-semibold text-accent-2'
                  : 'border-line text-dim hover:border-line-2 hover:text-text',
              )}
            >
              {t(`catalogue.gap.${gap}`)}
            </button>
          )
        })}
      </div>

      <Rows
        rows={rows.data}
        pending={rows.isPending}
        failed={rows.isError}
        filter={filter}
        sort={sort}
        onReorder={reorder}
        onClearFilters={() => setFilter({})}
        onSelect={onSelect}
        selected={selected}
        onSelectionChange={setSelected}
        onDelete={(workIds) => remove.mutate(workIds)}
        deleting={remove.isPending}
      />
    </div>
  )
}

function Rows({
  rows,
  pending,
  failed,
  filter,
  sort,
  onReorder,
  onClearFilters,
  onSelect,
  selected,
  onSelectionChange,
  onDelete,
  deleting,
}: {
  rows: ScoredWork[] | undefined
  pending: boolean
  failed: boolean
  filter: CatalogueFilter
  sort: Sort
  onReorder: (column: SortColumn) => void
  onClearFilters: () => void
  onSelect: (workId: string, tab?: Tab) => void
  selected: ReadonlySet<string>
  onSelectionChange: (next: ReadonlySet<string>) => void
  onDelete: (workIds: readonly string[]) => void
  deleting: boolean
}) {
  const { t } = useTranslation()
  const profile = useProfile()

  if (pending) return <SkeletonList rows={6} />

  if (failed || rows === undefined) {
    return (
      <p role="alert" className="text-sm text-bad">
        {t('toast.loadFailed')}
      </p>
    )
  }

  // An empty profile and an over-narrow filter look the same and mean opposite
  // things: one asks you to write something, the other to stop hiding it.
  if (rows.length === 0) {
    return <EmptyState title={t('empty.worksTitle')} body={t('empty.worksBody')} />
  }

  const visible = sortRows(narrow(rows, filter), sort)
  const narrowed = isNarrowed(filter)

  // Only what is on screen can be ticked by the header box: filtering something
  // out and then selecting "all" must not reach it.
  const shownIds = visible.map((row) => row.work_id)
  const chosen = shownIds.filter((id) => selected.has(id))
  const allChosen = chosen.length > 0 && chosen.length === shownIds.length

  const toggleRow = (workId: string) => {
    const next = new Set(selected)
    if (!next.delete(workId)) next.add(workId)
    onSelectionChange(next)
  }

  const toggleAll = () => onSelectionChange(new Set(allChosen ? [] : shownIds))

  // One list, read by both ways into a row: the three dots and the right
  // click. Written once so the two can never come to disagree about what a
  // row can do.
  const actionsFor = (row: ScoredWork): RowAction[] => [
    {
      key: 'score',
      label: t('catalogue.action.score'),
      onSelect: () => onSelect(row.work_id, 'score'),
    },
    {
      key: 'schedule',
      label: t('catalogue.action.schedule'),
      onSelect: () => onSelect(row.work_id, 'releases'),
    },
    {
      key: 'delete',
      label: t('catalogue.action.delete'),
      danger: true,
      onSelect: () => onDelete([row.work_id]),
    },
  ]

  if (visible.length === 0) {
    return (
      <EmptyState
        title={t('empty.worksFiltered')}
        body={t('empty.worksFilteredBody')}
        action={<Button onClick={onClearFilters}>{t('empty.clearFilters')}</Button>}
      />
    )
  }

  return (
    <div className="flex flex-col gap-2">
      {/* Only while rows are ticked. It replaces nothing and hides nothing — the
          table stays exactly where it was, so the next click is on the row you
          were already looking at. */}
      {chosen.length > 0 && (
        <div className="flex flex-wrap items-center gap-3 rounded-[10px] border border-line bg-raise px-3 py-2 text-sm">
          <span className="font-medium">{t('catalogue.chosen', { count: chosen.length })}</span>
          <BulkActions workIds={chosen} onStarted={() => onSelectionChange(new Set())} />
          <Button
            variant="danger"
            size="sm"
            disabled={deleting}
            onClick={() => onDelete(chosen)}
          >
            {t('catalogue.action.delete')}
          </Button>
          <button
            type="button"
            onClick={() => onSelectionChange(new Set())}
            className="ml-auto cursor-pointer text-xs text-dim underline decoration-dotted underline-offset-2 transition-colors hover:text-text"
          >
            {t('catalogue.clearSelection')}
          </button>
        </div>
      )}

      {/* Only while something is hidden: "3 of 3" on every screen is noise, and
          the number that matters is how much you are not seeing. */}
      {narrowed && (
        <div className="flex items-center gap-2 text-xs text-dim">
          <Badge variant="soft">
            {t('catalogue.showing', { shown: visible.length, total: rows.length })}
          </Badge>
          <button
            type="button"
            onClick={onClearFilters}
            className="cursor-pointer underline decoration-dotted underline-offset-2 transition-colors hover:text-text"
          >
            {t('catalogue.clear')}
          </button>
        </div>
      )}

      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-line text-left text-xs uppercase tracking-wide text-dim">
            <th className="w-9 py-2">
              <input
                type="checkbox"
                className="size-3.5 cursor-pointer accent-[var(--accent)]"
                checked={allChosen}
                // Some but not all: the box shows neither state, because it is
                // neither, and clicking it takes the rest.
                ref={(box) => {
                  if (box) box.indeterminate = chosen.length > 0 && !allChosen
                }}
                onChange={toggleAll}
                aria-label={t('catalogue.selectAll')}
              />
            </th>
            <Column column="title" sort={sort} onReorder={onReorder} label={t('catalogue.work')} />
            <Column column="tier" sort={sort} onReorder={onReorder} label={t('catalogue.tier')} />
            <Column
              column="total"
              sort={sort}
              onReorder={onReorder}
              label={t('catalogue.total')}
              align="right"
            />
            <Column
              column="scored"
              sort={sort}
              onReorder={onReorder}
              label={t('catalogue.scored')}
            />
            <th className="w-10 py-2" />
          </tr>
        </thead>
        <tbody>
          {visible.map((row) => (
            <Row
              key={row.work_id}
              actions={actionsFor(row)}
              onOpen={() => onSelect(row.work_id)}
            >
              <td className="py-2" onClick={(event) => event.stopPropagation()}>
                <input
                  type="checkbox"
                  className="size-3.5 cursor-pointer accent-[var(--accent)]"
                  checked={selected.has(row.work_id)}
                  onChange={() => toggleRow(row.work_id)}
                  aria-label={t('catalogue.select', { title: row.title })}
                />
              </td>
              <td className="px-3 py-2">
                <span className="font-medium">{row.title}</span>
                <span className="ml-2 text-xs text-dim">
                  {labelOf(profile.config.statuses, row.status)} ·{' '}
                  {labelOf(profile.config.work_kinds, row.kind)}
                </span>
              </td>
              <td className="px-3 py-2">
                {row.tier === null ? (
                  <span className="text-faint">—</span>
                ) : (
                  <span className="rounded bg-accent-soft px-1.5 py-0.5 text-xs">
                    {labelOf(profile.config.tiers, row.tier)}
                  </span>
                )}
              </td>
              <td
                className={cn(
                  'px-3 py-2 text-right tabular-nums',
                  row.total === null && 'text-faint',
                )}
              >
                {row.total?.toFixed(1) ?? '—'}
              </td>
              <td className="px-3 py-2 text-xs text-dim">
                {row.scored_at?.slice(0, 10) ?? '—'}
                {row.stale && (
                  <span
                    className="ml-2 rounded bg-warn-soft px-1.5 py-0.5 text-warn"
                    title={t('catalogue.staleHint')}
                  >
                    {t('catalogue.stale')}
                  </span>
                )}
              </td>
              <td className="py-2 text-right">
                <RowMenu
                  label={t('catalogue.rowMenu', { title: row.title })}
                  actions={actionsFor(row)}
                />
              </td>
            </Row>
          ))}
        </tbody>
      </table>
    </div>
  )
}

/**
 * One row of the catalogue, with both ways into its actions.
 *
 * The right click is `RowContextMenu` rendering the `<tr>` itself rather than
 * wrapping it: a `<div>` between `<tbody>` and `<tr>` is invalid markup, and
 * the browser repairs it by lifting the row out of the table.
 */
function Row({
  actions,
  onOpen,
  children,
}: {
  actions: RowAction[]
  onOpen: () => void
  children: ReactNode
}) {
  return (
    <RowContextMenu
      actions={actions}
      render={
        <tr
          className={cn(
            'cursor-pointer border-b border-line hover:bg-soft',
            // While its menu is open the row stays lit, so it is obvious which
            // of twenty rows the actions belong to.
            'data-[popup-open]:bg-soft',
          )}
          onClick={onOpen}
        />
      }
    >
      {children}
    </RowContextMenu>
  )
}

/** A column header that sorts, and says which way it is pointing. */
function Column({
  column,
  sort,
  onReorder,
  label,
  align = 'left',
}: {
  column: SortColumn
  sort: Sort
  onReorder: (column: SortColumn) => void
  label: string
  align?: 'left' | 'right'
}) {
  const { t } = useTranslation()
  const active = sort.column === column
  const Arrow = sort.direction === 'asc' ? ArrowUp : ArrowDown

  return (
    // `aria-sort` goes on the cell rather than the button: it describes the
    // column, and a button is not a column.
    <th
      aria-sort={active ? (sort.direction === 'asc' ? 'ascending' : 'descending') : 'none'}
      // Padding on both sides, not just the one the text runs towards: a
      // right-aligned column followed by a left-aligned one used to put its
      // last letter against the next header's first, and "Total"/"Scored" read
      // as one word.
      className={cn('px-3 py-2 font-medium')}
    >
      <button
        type="button"
        onClick={() => onReorder(column)}
        title={t('catalogue.sortBy', { column: label })}
        className={cn(
          'inline-flex cursor-pointer items-center gap-1 uppercase tracking-wide transition-colors hover:text-text',
          align === 'right' && 'w-full justify-end',
          active && 'text-text',
        )}
      >
        {label}
        {active && <Arrow aria-hidden className="size-3" />}
      </button>
    </th>
  )
}
