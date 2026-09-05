import { type ReactNode, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ArrowDown, ArrowUp, Check, ChevronRight, Columns3 } from 'lucide-react'
import {
  catalogue as fetchCatalogue,
  createWork,
  deleteWorks,
  setWorksStatus,
  unscheduleWorks,
  type ScoredWork,
} from '@/lib/api'
import {
  ALL_COLUMNS,
  GAPS,
  groupRows,
  isNarrowed,
  loadColumns,
  loadFilter,
  loadSort,
  narrow,
  REQUIRED_COLUMN,
  saveColumns,
  saveFilter,
  saveSort,
  sortRows,
  toggleColumn,
  toggleSort,
  type CatalogueFilter,
  type ColumnId,
  type GroupBy,
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
import {
  Menu,
  MenuCheckboxIndicator,
  MenuCheckboxItem,
  MenuPopup,
  MenuTrigger,
} from '@/components/ui/menu'
import { RowContextMenu, RowMenu, type RowAction } from '@/components/ui/RowMenu'
import { SkeletonList } from '@/components/ui/Skeleton'
import { cn } from '@/lib/utils'

interface Props {
  /** Opens a work, optionally straight onto one of its tabs. */
  onSelect: (workId: string, tab?: Tab) => void
}

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
  // Held for the session rather than for the moment: leaving for a card and
  // coming back is the commonest thing anyone does here, and a filter that
  // does not survive it makes the catalogue hostile to its own use.
  const [filter, setFilterState] = useState<CatalogueFilter>(loadFilter)

  const setFilter = (next: CatalogueFilter) => {
    setFilterState(next)
    saveFilter(next)
  }
  const [sort, setSort] = useState<Sort>(loadSort)
  // Which columns are shown outlives a restart, like the sort: it is how a
  // person reads the table, not what they are doing this minute.
  const [columns, setColumnsState] = useState<ColumnId[]>(loadColumns)

  const setColumns = (next: ColumnId[]) => {
    setColumnsState(next)
    saveColumns(next)
  }

  // Grouping is deliberately of the moment, like the filter and unlike the
  // sort: it is a way of interrogating the list today, and finding the
  // catalogue folded into blocks tomorrow reads as something being wrong.
  const [groupBy, setGroupBy] = useState<GroupBy>('none')
  const [collapsed, setCollapsed] = useState<ReadonlySet<string>>(new Set())
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
    // One call, not one per work: a loop here left the journal with a line
    // apiece and could stop halfway with nothing to say where.
    mutationFn: (workIds: readonly string[]) => deleteWorks([...workIds]),
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

  // The catalogue reloads either way, so both of these report a count rather
  // than patching rows: what a person wants told is how many it reached, and
  // "skipped" is why the number can be smaller than what they ticked.
  const refreshAfterBulk = () => {
    setSelected(new Set())
    void client.invalidateQueries({ queryKey: keys.works })
    void client.invalidateQueries({ queryKey: keys.catalogue })
    void client.invalidateQueries({ queryKey: keys.workspace })
    void client.invalidateQueries({ queryKey: keys.journal })
  }

  const restatus = useMutation({
    mutationFn: ({ workIds, status }: { workIds: readonly string[]; status: string }) =>
      setWorksStatus([...workIds], status),
    onSuccess: (outcome) => {
      refreshAfterBulk()
      say.ok(t('catalogue.bulk.statusSet', { count: outcome.changed }))
    },
    onError: (cause) => say.failedTo(t('toast.workSaveFailed'), cause),
  })

  const unschedule = useMutation({
    mutationFn: (workIds: readonly string[]) => unscheduleWorks([...workIds]),
    onSuccess: (outcome) => {
      refreshAfterBulk()
      void client.invalidateQueries({ queryKey: keys.calendar })
      say.ok(t('catalogue.bulk.unscheduled', { count: outcome.changed }))
    },
    onError: (cause) => say.failedTo(t('toast.workSaveFailed'), cause),
  })

  const reorder = (column: SortColumn) => {
    const next = toggleSort(sort, column)
    setSort(next)
    saveSort(next)
  }

  const set = (change: Partial<CatalogueFilter>) =>
    setFilter({ ...filter, ...change })

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
        <div className="ml-auto flex items-center gap-2">
          <Select
            className="w-40"
            aria-label={t('catalogue.groupBy')}
            value={groupBy === 'none' ? '' : groupBy}
            onChange={(value) => {
              setGroupBy((value || 'none') as GroupBy)
              // Blocks from the previous grouping mean nothing under the new
              // one, and a block folded shut for a reason nobody remembers is
              // just a row that went missing.
              setCollapsed(new Set())
            }}
            placeholder={t('catalogue.groupNone')}
            options={[
              { value: 'status', label: t('catalogue.groupStatus') },
              { value: 'tier', label: t('catalogue.groupTier') },
            ]}
          />
          <ColumnPicker columns={columns} onChange={setColumns} />
        </div>
      </div>

      <Rows
        rows={rows.data}
        pending={rows.isPending}
        failed={rows.isError}
        filter={filter}
        sort={sort}
        columns={columns}
        groupBy={groupBy}
        collapsed={collapsed}
        onToggleGroup={(key) => {
          const next = new Set(collapsed)
          if (!next.delete(key)) next.add(key)
          setCollapsed(next)
        }}
        onReorder={reorder}
        onClearFilters={() => setFilter({})}
        onSelect={onSelect}
        selected={selected}
        onSelectionChange={setSelected}
        onDelete={(workIds) => remove.mutate(workIds)}
        deleting={remove.isPending}
        onSetStatus={(workIds, status) => restatus.mutate({ workIds, status })}
        onUnschedule={(workIds) => unschedule.mutate(workIds)}
        busy={restatus.isPending || unschedule.isPending}
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
  columns,
  groupBy,
  collapsed,
  onToggleGroup,
  onReorder,
  onClearFilters,
  onSelect,
  selected,
  onSelectionChange,
  onDelete,
  deleting,
  onSetStatus,
  onUnschedule,
  busy,
}: {
  rows: ScoredWork[] | undefined
  pending: boolean
  failed: boolean
  filter: CatalogueFilter
  sort: Sort
  columns: ColumnId[]
  groupBy: GroupBy
  collapsed: ReadonlySet<string>
  onToggleGroup: (key: string) => void
  onReorder: (column: SortColumn) => void
  onClearFilters: () => void
  onSelect: (workId: string, tab?: Tab) => void
  selected: ReadonlySet<string>
  onSelectionChange: (next: ReadonlySet<string>) => void
  onDelete: (workIds: readonly string[]) => void
  deleting: boolean
  onSetStatus: (workIds: readonly string[], status: string) => void
  onUnschedule: (workIds: readonly string[]) => void
  busy: boolean
}) {
  const { t } = useTranslation()
  const profile = useProfile()
  // Where the last plain tick landed, so a shift-click has something to reach
  // back to. A ref rather than state: it changes what the *next* click means
  // and nothing on screen depends on it. Declared above the early returns
  // below, because a hook that runs only sometimes is not a hook.
  const anchor = useRef<string | null>(null)

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
  const blocks = groupRows(visible, groupBy)

  // The label a block carries. A status or tier the profile has since dropped
  // still names its block by its bare key rather than vanishing — the works are
  // real even when the vocabulary moved on.
  const groupLabel = (key: string | null) => {
    if (key === null) return t('catalogue.groupNone')
    return groupBy === 'status'
      ? labelOf(profile.config.statuses, key)
      : labelOf(profile.config.tiers, key)
  }

  // Only what is on screen can be ticked by the header box: filtering something
  // out and then selecting "all" must not reach it.
  const shownIds = visible.map((row) => row.work_id)
  const chosen = shownIds.filter((id) => selected.has(id))
  const allChosen = chosen.length > 0 && chosen.length === shownIds.length

  const toggleRow = (workId: string, extend: boolean) => {
    const next = new Set(selected)

    // Shift-click takes everything between, in the order the table is showing —
    // which is the order a person sees and means, not the order the ids happen
    // to be in.
    const from = anchor.current
    if (extend && from !== null && from !== workId) {
      const start = shownIds.indexOf(from)
      const end = shownIds.indexOf(workId)
      if (start !== -1 && end !== -1) {
        const [lo, hi] = start < end ? [start, end] : [end, start]
        // The anchor's own state decides the run: dragging out of a selection
        // clears the span, dragging out of an empty one fills it.
        const adding = selected.has(from)
        for (const id of shownIds.slice(lo, hi + 1)) {
          if (adding) next.add(id)
          else next.delete(id)
        }
        onSelectionChange(next)
        return
      }
    }

    if (!next.delete(workId)) next.add(workId)
    anchor.current = workId
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

          {/* The ordinary things, beside the assistant's. Until now this bar
              could ask Claude to critique twenty works but could not move them
              to another status — the everyday act was the missing one. */}
          <Select
            className="w-40"
            aria-label={t('catalogue.bulk.setStatus')}
            // Always reads as the placeholder: it asks a question rather than
            // reporting a state, because a selection of twenty works has no one
            // status to show.
            value=""
            onChange={(value) => {
              // Guarded rather than disabled: the primitive takes no `disabled`,
              // and a second click while the first is in flight would ask the
              // same thing twice.
              if (value && !busy) onSetStatus(chosen, value)
            }}
            placeholder={t('catalogue.bulk.setStatus')}
            options={profile.config.statuses.map((s) => ({ value: s.key, label: s.label }))}
          />
          <Button
            variant="ghost"
            size="sm"
            disabled={busy}
            onClick={() => onUnschedule(chosen)}
          >
            {t('catalogue.bulk.unschedule')}
          </Button>
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
            {columns.map((id) => {
              const spec = COLUMN_SPECS[id]
              return spec.sort === null ? (
                <th key={id} className={cn('px-3 py-2', spec.width)}>
                  {t(spec.label)}
                </th>
              ) : (
                <Column
                  key={id}
                  column={spec.sort}
                  sort={sort}
                  onReorder={onReorder}
                  label={t(spec.label)}
                  align={spec.align}
                />
              )
            })}
            <th className="w-10 py-2" />
          </tr>
        </thead>
        {blocks.map((block) => {
          const key = block.key ?? GROUPLESS
          const folded = groupBy !== 'none' && collapsed.has(key)

          return (
            <tbody key={key}>
              {groupBy !== 'none' && (
                <tr className="border-b border-line bg-soft/60">
                  <td colSpan={columns.length + 2} className="px-2 py-1.5">
                    <button
                      type="button"
                      onClick={() => onToggleGroup(key)}
                      aria-expanded={!folded}
                      className="flex cursor-pointer items-center gap-1.5 text-xs font-medium text-dim transition-colors hover:text-text"
                    >
                      <ChevronRight
                        className={cn('size-3.5 transition-transform', !folded && 'rotate-90')}
                        aria-hidden
                      />
                      <span>{groupLabel(block.key)}</span>
                      <span className="text-faint">{block.rows.length}</span>
                    </button>
                  </td>
                </tr>
              )}

              {!folded &&
                block.rows.map((row) => (
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
                        // The change carries no modifier, so the click does. A
                        // shift-click on a label also reaches the box, and both
                        // ways of ticking mean the same thing.
                        onClick={(event) => {
                          event.stopPropagation()
                          toggleRow(row.work_id, event.shiftKey)
                        }}
                        onChange={() => undefined}
                        aria-label={t('catalogue.select', { title: row.title })}
                      />
                    </td>

                    {columns.map((id) => (
                      <Cell key={id} column={id} row={row} />
                    ))}

                    <td className="py-2 text-right">
                      <RowMenu
                        label={t('catalogue.rowMenu', { title: row.title })}
                        actions={actionsFor(row)}
                      />
                    </td>
                  </Row>
                ))}
            </tbody>
          )
        })}
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

/** The key standing in for "no value", which a `Map` cannot hold as `null`. */
const GROUPLESS = ' none'

/** How one column is headed, sized and aligned. */
interface ColumnSpec {
  label: string
  /** The sort it drives, or null for a column that answers no ordering question. */
  sort: SortColumn | null
  align?: 'left' | 'right'
  width?: string
}

/**
 * Every column in one table, read by both the header and the body.
 *
 * The two used to be written out separately, and the fifth column of a header
 * could quietly describe the sixth cell of a row. Here a column is one entry
 * and cannot come apart.
 */
const COLUMN_SPECS: Record<ColumnId, ColumnSpec> = {
  id: { label: 'catalogue.column.id', sort: null, width: 'w-28' },
  title: { label: 'catalogue.work', sort: 'title' },
  marks: { label: 'catalogue.column.marks', sort: null, width: 'w-24' },
  versions: { label: 'catalogue.column.versions', sort: 'versions', align: 'right', width: 'w-16' },
  tier: { label: 'catalogue.tier', sort: 'tier' },
  total: { label: 'catalogue.total', sort: 'total', align: 'right' },
  scored: { label: 'catalogue.scored', sort: 'scored' },
  created: { label: 'catalogue.column.created', sort: 'created' },
  updated: { label: 'catalogue.column.updated', sort: 'updated' },
}

/** One cell, drawn from the column that asked for it. */
function Cell({ column, row }: { column: ColumnId; row: ScoredWork }) {
  const { t } = useTranslation()
  const profile = useProfile()

  switch (column) {
    case 'id':
      // Monospaced and dimmed: it is here to be copied and compared, not read
      // as part of the sentence a row makes.
      return <td className="px-3 py-2 font-mono text-xs text-faint">{row.work_id.slice(0, 8)}</td>

    case 'title':
      return (
        <td className="px-3 py-2">
          <span className="font-medium">{row.title}</span>
          <span className="ml-2 text-xs text-dim">
            {labelOf(profile.config.statuses, row.status)} {'·'}{' '}
            {labelOf(profile.config.work_kinds, row.kind)}
          </span>
        </td>
      )

    case 'marks': {
      // A mark the profile no longer defines is not drawn - the same rule the
      // card follows, so the two screens never disagree about what a work says.
      // A profile written before marks existed has none at all, and every mark
      // on every work is then unknown, which is the correct reading.
      const defined = profile.config.marks ?? []
      const shown = row.marks.filter((key) => defined.some((mark) => mark.key === key))
      return (
        <td className="px-3 py-2">
          {shown.length === 0 ? (
            <span className="text-faint">{'—'}</span>
          ) : (
            <span className="flex flex-wrap gap-1">
              {shown.map((key) => (
                <span
                  key={key}
                  className="rounded bg-soft px-1.5 py-0.5 text-[11px] text-dim"
                  title={labelOf(defined, key)}
                >
                  {labelOf(defined, key)}
                </span>
              ))}
            </span>
          )}
        </td>
      )
    }

    case 'versions':
      return (
        <td
          className={cn(
            'px-3 py-2 text-right tabular-nums',
            row.version_count === 0 && 'text-faint',
          )}
        >
          {row.version_count}
        </td>
      )

    case 'tier':
      return (
        <td className="px-3 py-2">
          {row.tier === null ? (
            <span className="text-faint">{'—'}</span>
          ) : (
            <span className="rounded bg-accent-soft px-1.5 py-0.5 text-xs">
              {labelOf(profile.config.tiers, row.tier)}
            </span>
          )}
        </td>
      )

    case 'total':
      return (
        <td className={cn('px-3 py-2 text-right tabular-nums', row.total === null && 'text-faint')}>
          {row.total?.toFixed(1) ?? '—'}
        </td>
      )

    case 'scored':
      return (
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
      )

    case 'created':
      return <td className="px-3 py-2 text-xs text-dim">{row.created_at.slice(0, 10)}</td>

    case 'updated':
      return <td className="px-3 py-2 text-xs text-dim">{row.updated_at.slice(0, 10)}</td>
  }
}

/**
 * Which columns the table draws.
 *
 * A menu rather than a dialog: choosing columns is something a person does
 * while looking at the table, and a dialog would cover the thing being changed.
 */
function ColumnPicker({
  columns,
  onChange,
}: {
  columns: ColumnId[]
  onChange: (next: ColumnId[]) => void
}) {
  const { t } = useTranslation()

  return (
    <Menu>
      <MenuTrigger
        render={
          <Button variant="icon" size="icon-sm" aria-label={t('catalogue.columns')} title={t('catalogue.columns')} />
        }
      >
        <Columns3 aria-hidden />
      </MenuTrigger>

      <MenuPopup align="end">
        {ALL_COLUMNS.map((id) => (
          <MenuCheckboxItem
            key={id}
            checked={columns.includes(id)}
            // The title carries the row's identity and its link. Offered as
            // permanently ticked rather than left out of the list, so its
            // absence is a statement instead of an oversight.
            disabled={id === REQUIRED_COLUMN}
            // The menu stays open: turning columns on and off is a comparison,
            // and closing after each would make a five-column change five trips.
            closeOnClick={false}
            onCheckedChange={() => onChange(toggleColumn(columns, id))}
          >
            <MenuCheckboxIndicator>
              <Check className="size-3.5" aria-hidden />
            </MenuCheckboxIndicator>
            {t(COLUMN_SPECS[id].label)}
          </MenuCheckboxItem>
        ))}
      </MenuPopup>
    </Menu>
  )
}
