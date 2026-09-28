import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { unscheduleWorks } from '@/lib/api/releases'
import { deleteWorks, setWorksStatus } from '@/lib/api/works'
import { updateProfileConfig } from '@/lib/api/workspace'
import {
  columnsForKind,
  loadColumnsChosen,
  saveColumnsChosen,
  withIntroduced,
  countGaps,
  isNarrowed,
  loadColumnFilters,
  loadFilter,
  loadSort,
  moveColumn,
  saveColumnFilters,
  saveFilter,
  saveSort,
  toggleSort,
  withColumns,
  type CatalogueFilter,
  type ColumnFilters,
  type ColumnId,
  type GroupBy,
  type Sort,
  type SortColumn,
} from '@/lib/catalogue'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { formatQuery, parseQuery, type Vocabulary } from '@/lib/searchQuery'
import { stagesOf } from '@/lib/stages'
import { say } from '@/lib/toast'
import { announceDeleted } from '@/lib/trash'
import { allOf, say as sayLabel, useProfile } from '@/lib/useProfile'
import {
  addView,
  loadViews,
  removeView,
  saveViews,
  viewOf,
  type SavedView,
  type ViewShape,
} from '@/lib/views'
import type { Tab } from '@/features/work/tabs'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { SkeletonList } from '@/components/ui/skeleton'
import { Frame } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { NewWorkDialog } from '@/components/NewWorkDialog'
import { BulkBar } from './BulkBar'
import { CatalogueTable } from './CatalogueTable'
import { CatalogueToolbar } from './CatalogueToolbar'
import { useShownRows } from './useShownRows'

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
 *
 * Laid out on the frame (ADR 0038): the toolbar's two rows stand at the top,
 * the table's panel takes the rest and scrolls inside, and the bar of bulk
 * actions stands at the foot while anything is ticked. This file holds the
 * state and the writes; what is drawn is in the files beside it.
 */
export function Catalogue({ onSelect }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const client = useQueryClient()
  // The kind the new-work dialog opens on, or null while it is closed.
  const [adding, setAdding] = useState<string | null>(null)
  // Held for the session rather than for the moment: leaving for a card and
  // coming back is the commonest thing anyone does here, and a filter that
  // does not survive it makes the catalogue hostile to its own use.
  const [filter, setFilterState] = useState<CatalogueFilter>(loadFilter)

  const setFilter = (next: CatalogueFilter) => {
    setFilterState(next)
    saveFilter(next)
  }
  // The header funnels: the same lifetime as the query filter, and beside it
  // rather than inside it, because the box writes the filter as one line and
  // a set of ticked stages has no place in the line.
  const [columnFilters, setColumnFiltersState] = useState<ColumnFilters>(loadColumnFilters)
  const setColumnFilters = (next: ColumnFilters) => {
    setColumnFiltersState(next)
    saveColumnFilters(next)
  }
  const [sort, setSort] = useState<Sort>(loadSort)
  // Which columns are shown is a fact about the craft, kept on the profile
  // since v0.50: a novel and a record are read down different columns.
  // The kind the table is narrowed to reads down its own columns, so the
  // list is derived from the filter rather than held once. What this screen
  // just chose is kept beside the profile's copy, by kind key ('' for no
  // kind): the table must not flick back to the old list between the click
  // and the profile coming back with the new one.
  const [chosenColumns, setChosenColumns] = useState<Record<string, ColumnId[]>>({})
  // Whether the columns v0.79 added may still be shown to a list stored
  // before them - see `withIntroduced`.
  const [columnsChosen, setColumnsChosen] = useState(loadColumnsChosen)
  const kindKey = filter.kind ?? ''
  const columns =
    chosenColumns[kindKey] ??
    withIntroduced(columnsForKind(profile.config, filter.kind), columnsChosen)

  const keepColumns = useAppMutation({
    mutationFn: ({ kind, next }: { kind: string | undefined; next: ColumnId[] }) =>
      updateProfileConfig(profile.id, {
        ...profile.config,
        ...withColumns(profile.config, kind, next),
      }),
    failure: 'toast.profileSaveFailed',
    refresh: refresh.profile,
  })

  const setColumns = (next: ColumnId[]) => {
    setChosenColumns((current) => ({ ...current, [kindKey]: next }))
    setColumnsChosen(true)
    saveColumnsChosen()
    keepColumns.mutate({ kind: filter.kind, next })
  }

  // Grouping is deliberately of the moment, like the filter and unlike the
  // sort: it is a way of interrogating the list today, and finding the
  // catalogue folded into blocks tomorrow reads as something being wrong.
  const [groupBy, setGroupByState] = useState<GroupBy>('none')
  const [collapsed, setCollapsed] = useState<ReadonlySet<string>>(new Set())
  const setGroupBy = (next: GroupBy) => {
    setGroupByState(next)
    // Blocks from the previous grouping mean nothing under the new one, and a
    // block folded shut for a reason nobody remembers is just a row that went
    // missing.
    setCollapsed(new Set())
  }

  const rows = useQuery(queries.catalogue())
  // The calendar's bookings, for the next release and whether it is ready.
  // Not waited for: the table draws without them and fills the two columns in.
  const releases = useQuery(queries.calendar())
  const shown = useShownRows(rows.data, releases.data, filter, columnFilters, sort)

  const kindCounts = new Map<string, number>()
  for (const row of rows.data ?? []) kindCounts.set(row.kind, (kindCounts.get(row.kind) ?? 0) + 1)

  // Deliberately not remembered across a restart, unlike the sort: a selection
  // is about the click you are about to make, and finding rows still ticked
  // tomorrow is a way to act on the wrong ones.
  const [selected, setSelected] = useState<ReadonlySet<string>>(new Set())
  // What the bar acts on: the ticked rows that are on show. Narrowing a
  // ticked row out of sight takes it out of the batch - an action must never
  // reach a work nobody can see.
  const chosen = (shown?.visible ?? []).map((row) => row.work_id).filter((id) => selected.has(id))
  const clearSelection = () => setSelected(new Set())

  const remove = useAppMutation({
    // One call, not one per work: a loop here left the journal with a line
    // apiece and could stop halfway with nothing to say where.
    mutationFn: (workIds: readonly string[]) => deleteWorks([...workIds]),
    failure: 'toast.workSaveFailed',
    onSuccess: (discarded) => {
      clearSelection()
      announceDeleted({
        client,
        deletionIds: discarded.entries,
        message: t('catalogue.deleted', { count: discarded.entries.length }),
        refresh: [keys.works, keys.catalogue, keys.workspace],
      })
      say.skipped(discarded.skipped)
    },
  })

  // The catalogue reloads either way, so both of these report a count rather
  // than patching rows: what a person wants told is how many it reached, and
  // "skipped" is why the number can be smaller than what they ticked.
  const restatus = useAppMutation({
    mutationFn: ({ workIds, status }: { workIds: readonly string[]; status: string }) =>
      setWorksStatus([...workIds], status),
    failure: 'toast.workSaveFailed',
    refresh: [keys.works, keys.catalogue, keys.workspace],
    onSuccess: (outcome) => {
      clearSelection()
      say.ok(t('catalogue.bulk.statusSet', { count: outcome.changed }))
      say.skipped(outcome.skipped)
    },
  })

  const unschedule = useAppMutation({
    mutationFn: (workIds: readonly string[]) => unscheduleWorks([...workIds]),
    failure: 'toast.workSaveFailed',
    refresh: refresh.works,
    onSuccess: (outcome) => {
      clearSelection()
      say.ok(t('catalogue.bulk.unscheduled', { count: outcome.changed }))
      say.skipped(outcome.skipped)
    },
  })

  const reorder = (column: SortColumn) => {
    const next = toggleSort(sort, column)
    setSort(next)
    saveSort(next)
  }

  // The profile's own words, which the query box resolves values against so
  // `tier:Picture` works as well as `tier:pic`.
  const vocabulary: Vocabulary = {
    statuses: allOf(profile.config, 'statuses').map((entry) => ({
      ...entry,
      label: sayLabel(entry.label),
    })),
    kinds: profile.config.work_kinds.map((entry) => ({ ...entry, label: sayLabel(entry.label) })),
    tiers: allOf(profile.config, 'tiers').map((entry) => ({
      ...entry,
      label: sayLabel(entry.label),
    })),
    stages: stagesOf(profile.config).map((entry) => ({ ...entry, label: sayLabel(entry.label) })),
  }

  // What the box shows. Held apart from the filter rather than derived from it,
  // because a half-typed `tier:cl` has no filter to be derived from and must
  // still stay on screen while it is being typed.
  const [query, setQuery] = useState(() => formatQuery(filter))
  const [unknown, setUnknown] = useState<{ field: string; value: string }[]>([])

  const runQuery = (line: string) => {
    setQuery(line)
    const parsed = parseQuery(line, vocabulary)
    setUnknown(parsed.unknown)
    // The gap chips are not part of the line, so they survive it: they have
    // their own row of controls and clearing the box must not turn them off.
    setFilter({
      ...parsed.filter,
      search: parsed.text === '' ? undefined : parsed.text,
      gap: filter.gap,
    })
  }

  // A chip writes the filter and the box has to follow, or the two ways of
  // narrowing would show different things about the same table.
  const setFromControl = (change: Partial<CatalogueFilter>) => {
    const next = { ...filter, ...change }
    setFilter(next)
    setQuery(formatQuery(next))
    setUnknown([])
  }

  const [views, setViewsState] = useState<SavedView[]>(loadViews)

  const setViews = (next: SavedView[]) => {
    setViewsState(next)
    saveViews(next)
  }

  const shape: ViewShape = { filter, sort, groupBy }

  const openView = (view: SavedView) => {
    setFilter(view.filter)
    setQuery(formatQuery(view.filter))
    setUnknown([])
    setSort(view.sort)
    saveSort(view.sort)
    setGroupBy(view.groupBy)
    // A view puts the catalogue back exactly as it was, and a funnel left
    // over from before would make it a different question.
    setColumnFilters({})
  }

  const clearFilters = () => {
    setFilter({})
    setColumnFilters({})
    setQuery('')
    setUnknown([])
  }

  const busy = restatus.isPending || unschedule.isPending || remove.isPending

  return (
    <Frame
      head={
        <CatalogueToolbar
          filter={filter}
          onControl={setFromControl}
          onGap={(gap) => setFilter({ ...filter, gap })}
          query={query}
          onQuery={runQuery}
          unknown={unknown[0]}
          views={views}
          shape={shape}
          onOpenView={openView}
          onSaveView={(name) => setViews(addView(views, viewOf(name, shape)))}
          onRemoveView={(id) => setViews(removeView(views, id))}
          columns={columns}
          onColumns={setColumns}
          onMoveColumn={(id, to) => setColumns(moveColumn(columns, id, to))}
          groupBy={groupBy}
          onGroupBy={setGroupBy}
          kindCounts={kindCounts}
          gapCounts={rows.data === undefined ? undefined : countGaps(rows.data)}
          shown={shown?.visible.length}
          total={shown?.all.length}
          narrowed={isNarrowed(filter, columnFilters)}
          chosen={chosen.length}
          onClearFilters={clearFilters}
        />
      }
      foot={
        chosen.length === 0 ? undefined : (
          <BulkBar
            workIds={chosen}
            busy={busy}
            onSetStatus={(status) => restatus.mutate({ workIds: chosen, status })}
            onUnschedule={() => unschedule.mutate(chosen)}
            onDelete={() => remove.mutate(chosen)}
            onClear={clearSelection}
          />
        )
      }
    >
      {/* An empty profile and an over-narrow filter look the same and mean
          opposite things: one asks you to write something, the other to stop
          hiding it. The first is this invitation; the second is drawn inside
          the table, under headings that stay. */}
      <Loaded
        query={rows}
        fill
        skeleton={<SkeletonList rows={6} />}
        isEmpty={(data) => data.length === 0}
        emptyState={
          <EmptyState
            title={t('empty.worksTitle')}
            body={t('empty.worksBody')}
            action={
              <Button
                variant="primary"
                size="sm"
                onClick={() => setAdding(profile.config.work_kinds[0]?.key ?? null)}
              >
                {t('empty.worksAction')}
              </Button>
            }
            className="flex-1"
          />
        }
      >
        {() =>
          shown === undefined ? null : (
            <CatalogueTable
              rows={shown.visible}
              columns={columns}
              sort={sort}
              onReorder={reorder}
              columnFilters={columnFilters}
              onColumnFilters={setColumnFilters}
              groupBy={groupBy}
              collapsed={collapsed}
              onToggleGroup={(key) => {
                const next = new Set(collapsed)
                if (!next.delete(key)) next.add(key)
                setCollapsed(next)
              }}
              kindNarrowed={filter.kind !== undefined}
              selected={selected}
              onSelectionChange={setSelected}
              onSelect={onSelect}
              onDelete={(workIds) => remove.mutate(workIds)}
              onClearFilters={clearFilters}
            />
          )
        }
      </Loaded>

      {/* The empty catalogue's way out: the same dialog the title bar's New
          opens, on the first kind, with the kind a field inside it. */}
      <NewWorkDialog kind={adding} onClose={() => setAdding(null)} onCreated={onSelect} />
    </Frame>
  )
}
