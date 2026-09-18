import { type ReactNode, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import {
  ArrowDown,
  ArrowUp,
  Bookmark,
  BookmarkPlus,
  Check,
  ChevronDown,
  ChevronRight,
  Columns3,
  Star,
  X,
} from 'lucide-react'
import {
  catalogue as fetchCatalogue,
  deleteWorks,
  setWorksStatus,
  unscheduleWorks,
  updateProfileConfig,
  type ScoredWork,
} from '@/lib/api'
import {
  ALL_COLUMNS,
  columnsFor,
  columnsForKind,
  withColumns,
  GAPS,
  groupRows,
  isNarrowed,
  loadFilter,
  loadSort,
  narrow,
  REQUIRED_COLUMN,
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
import { StagePicker } from '@/components/StagePicker'
import { stagesOf } from '@/lib/stages'
import { worksMatching } from '@/lib/api'
import { formatQuery, parseQuery, type Vocabulary } from '@/lib/searchQuery'
import {
  addView,
  loadViews,
  matchesView,
  removeView,
  saveViews,
  viewOf,
  type SavedView,
  type ViewShape,
} from '@/lib/views'
import { useDebounced } from '@/lib/useDebounced'
import { keys } from '@/lib/query'
import { coverImageFor } from '@/lib/cover'
import { useCovers } from '@/lib/useCovers'
import { announceDeleted } from '@/lib/trash'
import { say } from '@/lib/toast'
import { allOf, labelOf, useProfile, vocabularyOf } from '@/lib/useProfile'
import { nextTier } from '@/lib/scoring'
import { Pin } from 'lucide-react'
import type { Tab } from '@/components/card/tabs'
import { BulkActions } from '@/components/assistant/BulkActions'
import { Badge } from '@/components/ui/badge'
import { badgeVariantOf, markIconOf } from '@/lib/markIcon'
import { useStar } from '@/lib/useStar'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/EmptyState'
import { Input } from '@/components/ui/input'
import { Select } from '@/components/ui/AppSelect'
import {
  Menu,
  MenuCheckboxIndicator,
  MenuCheckboxItem,
  MenuItem,
  MenuPopup,
  MenuSeparator,
  MenuSub,
  MenuSubTrigger,
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
  // Which columns are shown is a fact about the craft, kept on the profile
  // since v0.50: a novel and a record are read down different columns. A
  // workspace from before the field opens on what this machine remembered,
  // and that list is written to the profile once so the move is invisible.
  const [opened] = useState(() => columnsFor(profile.config.catalogue_columns))
  // The kind the table is narrowed to reads down its own columns, so the
  // list is derived from the filter rather than held once. What this screen
  // just chose is kept beside the profile's copy, by kind key ('' for no
  // kind): the table must not flick back to the old list between the click
  // and the profile coming back with the new one.
  const [chosen, setChosen] = useState<Record<string, ColumnId[]>>({})
  const kindKey = filter.kind ?? ''
  const columns = chosen[kindKey] ?? columnsForKind(profile.config, filter.kind).columns

  const keepColumns = useMutation({
    mutationFn: ({ kind, next }: { kind: string | undefined; next: ColumnId[] }) =>
      updateProfileConfig(profile.id, {
        ...profile.config,
        ...withColumns(profile.config, kind, next),
      }),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: keys.workspace })
      void client.invalidateQueries({ queryKey: keys.profiles })
    },
    onError: (cause) => say.failedTo(t('toast.profileSaveFailed'), cause),
  })

  const setColumns = (next: ColumnId[]) => {
    setChosen((current) => ({ ...current, [kindKey]: next }))
    keepColumns.mutate({ kind: filter.kind, next })
  }

  useEffect(() => {
    if (opened.fromMachine) keepColumns.mutate({ kind: undefined, next: opened.columns })
    // Once, on the first open of a profile that has no columns yet.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  // Grouping is deliberately of the moment, like the filter and unlike the
  // sort: it is a way of interrogating the list today, and finding the
  // catalogue folded into blocks tomorrow reads as something being wrong.
  const [groupBy, setGroupBy] = useState<GroupBy>('none')
  const [collapsed, setCollapsed] = useState<ReadonlySet<string>>(new Set())

  const rows = useQuery({
    queryKey: keys.catalogue,
    queryFn: fetchCatalogue,
  })
  const kindCounts = new Map<string, number>()
  for (const row of rows.data ?? []) kindCounts.set(row.kind, (kindCounts.get(row.kind) ?? 0) + 1)


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

  // The profile's own words, which the query box resolves values against so
  // `tier:Picture` works as well as `tier:pic`.
  const vocabulary: Vocabulary = {
    statuses: allOf(profile.config, 'statuses'),
    kinds: profile.config.work_kinds,
    tiers: allOf(profile.config, 'tiers'),
    stages: stagesOf(profile.config),
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

  // A dropdown writes the filter and the box has to follow, or the two ways of
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
    setCollapsed(new Set())
  }

  const clearFilters = () => {
    setFilter({})
    setQuery('')
    setUnknown([])
  }

  return (
    <div className="flex h-full min-h-0 min-w-0 flex-col gap-4">
      {/* The kind of work as a row of chips, not one more dropdown: it is the
          mode the catalogue is in — songs, videos — and a mode is read at a
          glance and switched in one click. Hidden while the profile has one
          kind: "all" beside the only thing there is would be a choice of one.
          The counts are of the whole catalogue, so a kind reads as empty
          rather than as absent. */}
      {profile.config.work_kinds.length > 1 && (
        <div role="group" aria-label={t('works.kind')} className="flex flex-wrap items-center gap-2">
          {[
            { key: undefined, label: t('catalogue.kindAll'), count: rows.data?.length ?? 0 },
            ...profile.config.work_kinds.map((kind) => ({
              key: kind.key,
              label: kind.label,
              count: kindCounts.get(kind.key) ?? 0,
            })),
          ].map((entry) => {
            const active = filter.kind === entry.key
            return (
              <button
                key={entry.key ?? ''}
                type="button"
                aria-pressed={active}
                // The chip that is on turns off: back to every kind, without
                // a separate control for it.
                onClick={() => setFromControl({ kind: active ? undefined : entry.key })}
                className={cn(
                  'cursor-pointer rounded-full border px-2.5 py-0.5 text-[11.5px] transition-colors',
                  active
                    ? 'border-transparent bg-accent-soft font-semibold text-accent-2'
                    : 'border-line text-dim hover:border-line-2 hover:text-text',
                )}
              >
                {entry.label}
                <span className="ml-1.5 text-[10.5px] text-faint tabular-nums">{entry.count}</span>
              </button>
            )
          })}
        </div>
      )}

      <div className="flex flex-wrap items-center gap-2">
        <Input
          className="max-w-96 font-mono text-[12.5px]"
          value={query}
          onChange={(event) => runQuery(event.target.value)}
          placeholder={t('catalogue.queryPlaceholder')}
          aria-label={t('works.search')}
          aria-describedby="catalogue-query-help"
        />
        <Select
          className="w-44"
          aria-label={t('works.status')}
          value={filter.status ?? ''}
          onChange={(value) => setFromControl({ status: value || undefined })}
          placeholder={t('works.anyStatus')}
          options={allOf(profile.config, 'statuses').map((s) => ({ value: s.key, label: s.label }))}
        />
        <Select
          className="w-44"
          aria-label={t('catalogue.tier')}
          value={filter.tier ?? ''}
          onChange={(value) => setFromControl({ tier: value || undefined })}
          placeholder={t('catalogue.anyTier')}
          options={allOf(profile.config, 'tiers').map((tier) => ({ value: tier.key, label: tier.label }))}
        />
        {/* The starred, as one chip: the works marked to come back to. Not a
            token in the box — a star is raised and lowered with a click, and
            is asked for the same way. */}
        <button
          type="button"
          aria-pressed={filter.bookmarked === true}
          title={t('catalogue.starredHint')}
          onClick={() => setFromControl({ bookmarked: filter.bookmarked === true ? undefined : true })}
          className={cn(
            'inline-flex cursor-pointer items-center gap-1 rounded-full border px-2.5 py-0.5 text-[11.5px] transition-colors',
            filter.bookmarked === true
              ? 'border-transparent bg-warn-soft font-semibold text-warn'
              : 'border-line text-dim hover:border-line-2 hover:text-text',
          )}
        >
          <Star aria-hidden className={cn('size-3', filter.bookmarked === true && 'fill-current')} />
          {t('catalogue.starred')}
        </button>
      </div>

      {/* Says what the box can do without a doc, and says it once — the hint
          goes quiet the moment an operator is used, because by then it has been
          learned. A value the profile does not have is called out here rather
          than left to look like a search that found nothing. */}
      <p id="catalogue-query-help" className="-mt-2 text-[11.5px] text-faint">
        {unknown[0] !== undefined ? (
          <span className="text-bad">
            {t('catalogue.queryUnknown', {
              field: unknown[0].field,
              value: unknown[0].value,
            })}
          </span>
        ) : (
          t('catalogue.queryHint')
        )}
      </p>

      <ViewBar
        views={views}
        shape={shape}
        onOpen={openView}
        onSave={(name) => setViews(addView(views, viewOf(name, shape)))}
        onRemove={(id) => setViews(removeView(views, id))}
      />

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
        onClearFilters={clearFilters}
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

  // What the rows themselves cannot answer: which works say this somewhere
  // inside them - in a lyric, a note, a craft field, a reply from the
  // assistant. Debounced, because it is a round trip and the box is typed in.
  const text = filter.search?.trim() ?? ''
  const settled = useDebounced(text, 160)
  const matches = useQuery({
    queryKey: keys.worksMatching(settled),
    queryFn: () => worksMatching(settled),
    enabled: settled !== '',
    // The corpus does not change while a word is being typed, and every
    // keystroke that ends in the same query should cost nothing.
    staleTime: 30_000,
  })
  // While the answer for the current word is still on its way, the previous
  // one is worse than none: it would narrow to the hits for `холод` while the
  // box reads `холодильник`. Titles keep narrowing in the meantime.
  const matching =
    settled === '' || settled !== text || matches.data === undefined
      ? undefined
      : matches.data

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

  const visible = sortRows(narrow(rows, filter, matching), sort)
  const narrowed = isNarrowed(filter)
  const blocks = groupRows(visible, groupBy)

  // The label a block carries. A status or tier the profile has since dropped
  // still names its block by its bare key rather than vanishing — the works are
  // real even when the vocabulary moved on.
  const groupLabel = (key: string | null) => {
    if (key === null) return t('catalogue.groupNone')
    return groupBy === 'status'
      ? labelOf(allOf(profile.config, 'statuses'), key)
      : labelOf(allOf(profile.config, 'tiers'), key)
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
    <div className="flex h-full min-h-0 min-w-0 flex-col gap-2">
      {/* Only while rows are ticked. It replaces nothing and hides nothing — the
          table stays exactly where it was, so the next click is on the row you
          were already looking at. */}
      {chosen.length > 0 && (
        <div className="flex flex-wrap items-center gap-3 rounded-[10px] border border-line bg-raise px-3 py-2 text-sm">
          <span className="font-medium">{t('catalogue.chosen', { count: chosen.length })}</span>
          <BulkActions workIds={chosen} onStarted={() => onSelectionChange(new Set())} />

          {/* The ordinary things, behind one button beside the assistant's.
              Until now this bar could ask Claude to critique twenty works but
              could not move them to another status — the everyday act was the
              missing one. They sit in a menu rather than in the row because a
              dropdown among flat buttons read as the loudest thing here, and
              deleting must not be the easiest click to make by accident. */}
          <BulkMenu
            statuses={allOf(profile.config, 'statuses')}
            busy={busy || deleting}
            onSetStatus={(status) => onSetStatus(chosen, status)}
            onUnschedule={() => onUnschedule(chosen)}
            onDelete={() => onDelete(chosen)}
          />
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

      {/* The table scrolls sideways rather than being cut off by the window.
          Narrow, every column used to squeeze until dates broke across two
          lines and a row stood three lines tall; the last columns and the row
          menu were simply beyond the edge, unreachable.

          `min-w-0` is what makes the scrolling work at all. A flex child sizes
          to its content by default, so this box grew as wide as the table and
          never overflowed — while the screen container above clips the
          horizontal axis on purpose, to stop a trackpad swipe sliding content
          under the sidebar. The result was a table cut off at the edge with no
          scrollbar anywhere. Allowing this one box to be narrower than its
          contents puts the overflow, and the bar, inside the table.

          The box takes the height left on the screen and scrolls both axes
          itself, so the sideways bar sits at the bottom of the window instead
          of under the last of two hundred rows — where it was only reachable
          after scrolling to the very end of the list, which is no use to
          someone reading the middle. The header row is sticky for the same
          reason: a table you scroll is a table whose headings must stay. */}
      <div className="min-h-0 min-w-0 flex-1 overflow-auto">
        <table className="w-full min-w-max text-sm">
        {/* The headings stay while the rows move under them: a table long
            enough to need scrolling is one whose columns must remain named. */}
        <thead className="sticky top-0 z-10 bg-bg">
          <tr className="border-b border-line text-left text-xs uppercase tracking-wide text-dim">
            {/* The tick column carries the same side padding as every other
                cell: with none, the box sat flush against the star in the
                next one and the two read as one control. */}
            <th className="w-9 py-2 pl-3 pr-2">
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
                    <td
                      className="py-2 pl-3 pr-2"
                      onClick={(event) => event.stopPropagation()}
                    >
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
                      <Cell key={id} column={id} row={row} kindNarrowed={filter.kind !== undefined} />
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
    </div>
  )
}

/**
 * The star on a row: on when the work is marked to come back to, and a click
 * either way. It stops the row's own click, which would open the card.
 */
function RowStar({ row }: { row: ScoredWork }) {
  const { t } = useTranslation()
  const star = useStar(row.work_id)
  const on = row.bookmarked_at !== null
  return (
    <button
      type="button"
      aria-pressed={on}
      title={t(on ? 'work.unstar' : 'work.star')}
      disabled={star.isPending}
      onClick={(event) => {
        event.stopPropagation()
        star.mutate(!on)
      }}
      className={cn(
        'cursor-pointer rounded-md p-0.5 transition-colors',
        on ? 'text-warn' : 'text-faint/60 hover:text-dim',
      )}
    >
      <Star aria-hidden className={cn('size-3.5', on && 'fill-current')} />
    </button>
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
      className={cn('whitespace-nowrap px-3 py-2 font-medium')}
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
  stage: { label: 'catalogue.column.stage', sort: 'stage', width: 'w-14' },
  marks: { label: 'catalogue.column.marks', sort: null },
  versions: { label: 'catalogue.column.versions', sort: 'versions', align: 'right', width: 'w-16' },
  tier: { label: 'catalogue.tier', sort: 'tier' },
  total: { label: 'catalogue.total', sort: 'total', align: 'right' },
  scored: { label: 'catalogue.scored', sort: 'scored' },
  created: { label: 'catalogue.column.created', sort: 'created' },
  updated: { label: 'catalogue.column.updated', sort: 'updated' },
}

/** One cell, drawn from the column that asked for it. */
function Cell({
  column,
  row,
  kindNarrowed,
}: {
  column: ColumnId
  row: ScoredWork
  /** The table shows one kind of work: naming it on every row says nothing. */
  kindNarrowed: boolean
}) {
  const { t } = useTranslation()
  const profile = useProfile()
  const vocabulary = vocabularyOf(profile.config, row.kind)
  const covers = useCovers()

  switch (column) {
    case 'id':
      // Monospaced and dimmed: it is here to be copied and compared, not read
      // as part of the sentence a row makes.
      return <td className="px-3 py-2 font-mono text-xs text-faint">{row.work_id.slice(0, 8)}</td>

    case 'title': {
      const status = vocabulary.statuses.find((s) => s.key === row.status)
      return (
        // The title and what it is stay on one line. Squeezed, a two-word title
        // broke mid-phrase and the row grew to three lines; the table now
        // scrolls sideways instead of folding.
        <td className="whitespace-nowrap px-3 py-2">
          <span className="inline-flex items-center gap-2">
            <RowStar row={row} />
            {/* The cover as a chip beside the title rather than a column of
                its own: a column of pictures costs every row its height,
                and what the catalogue is read down is titles. */}
            <span
              aria-hidden
              className="size-5 shrink-0 rounded-[5px] border border-line/60"
              style={{ background: coverImageFor(row.work_id, covers.get(row.work_id)) }}
            />
            <span className="font-medium">{row.title}</span>
            {/* Where it stands as a badge in the status's own colour, and what
                it is in outline — read at a glance down the column, the way
                the header reads them. */}
            <Badge variant={badgeVariantOf(status?.colour)} className="px-2 text-[11px]">
              {status?.label ?? row.status}
            </Badge>
            {!kindNarrowed && (
              <Badge className="px-2 text-[11px]">
                {labelOf(profile.config.work_kinds, row.kind)}
              </Badge>
            )}
          </span>
        </td>
      )
    }

    case 'marks': {
      // A mark the profile no longer defines is not drawn - the same rule the
      // card follows, so the two screens never disagree about what a work says.
      // A profile written before marks existed has none at all, and every mark
      // on every work is then unknown, which is the correct reading.
      const defined = profile.config.marks ?? []
      const shown = row.marks.filter((key) => defined.some((mark) => mark.key === key))
      return (
        <td className="whitespace-nowrap px-3 py-2">
          {shown.length === 0 ? (
            <span className="text-faint">{'—'}</span>
          ) : (
            <span className="inline-flex flex-wrap gap-1">
              {shown.map((key) => {
                const mark = defined.find((m) => m.key === key)
                const Icon = markIconOf(mark ?? {})
                return (
                  <Badge
                    key={key}
                    variant={badgeVariantOf(mark?.colour ?? 'plain')}
                    className="gap-1 px-2 text-[11px]"
                    title={mark?.label ?? key}
                  >
                    <Icon aria-hidden className="size-3" />
                    {mark?.label ?? key}
                  </Badge>
                )
              })}
            </span>
          )}
        </td>
      )
    }

    case 'stage':
      return (
        // The dial and nothing else: a word per row would be a second column
        // of text beside the title, and the whole point of a dial is that a
        // column of them is read at a glance. The word is in the tooltip.
        <td className="px-3 py-2" onClick={(event) => event.stopPropagation()}>
          <StagePicker workId={row.work_id} percent={row.stage} compact />
        </td>
      )

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

    case 'tier': {
      // How far the next tier is, when the row has been judged at all. Only
      // the distance: which axis is cheapest needs the axis values, and a
      // catalogue row carries the total, not the score behind it. Naming the
      // axis here would mean shipping every work's axes to draw a table.
      const ahead =
        row.total === null || row.tier_pinned
          ? undefined
          : nextTier(vocabulary.tiers, row.total)

      return (
        <td className="px-3 py-2">
          {row.tier === null ? (
            <span className="text-faint">{'—'}</span>
          ) : (
            <span className="flex flex-wrap items-baseline gap-1.5">
              <span className="rounded bg-accent-soft px-1.5 py-0.5 text-xs">
                {labelOf(vocabulary.tiers, row.tier)}
              </span>
              {/* A tier held by hand is not a tier the score arrived at, and
                  a reader who cannot tell them apart is reading a number that
                  means two different things. */}
              {row.tier_pinned && (
                <Pin
                  aria-label={t('catalogue.tierPinned')}
                  className="size-3 shrink-0 self-center text-faint"
                />
              )}
              {ahead !== undefined && row.total !== null && (
                <span
                  className="text-[11px] text-faint tabular-nums"
                  title={t('catalogue.toNextTier', {
                    gap: (ahead.min - row.total).toFixed(1),
                    tier: ahead.label,
                  })}
                >
                  {`+${(ahead.min - row.total).toFixed(1)}`}
                </span>
              )}
            </span>
          )}
        </td>
      )
    }

    case 'total':
      return (
        <td className={cn('px-3 py-2 text-right tabular-nums', row.total === null && 'text-faint')}>
          {row.total?.toFixed(1) ?? '—'}
        </td>
      )

    case 'scored':
      return (
        // A date is one word. Left to wrap it broke into "2026-" over "07-31",
        // which reads as two dates rather than as one.
        <td className="whitespace-nowrap px-3 py-2 text-xs text-dim">
          {row.scored_at?.slice(0, 10) ?? '—'}
          {row.stale && (
            <span
              className="ml-2 whitespace-nowrap rounded bg-warn-soft px-1.5 py-0.5 text-warn"
              title={t('catalogue.staleHint')}
            >
              {t('catalogue.stale')}
            </span>
          )}
        </td>
      )

    case 'created':
      return (
        <td className="whitespace-nowrap px-3 py-2 text-xs text-dim">{row.created_at.slice(0, 10)}</td>
      )

    case 'updated':
      return (
        <td className="whitespace-nowrap px-3 py-2 text-xs text-dim">{row.updated_at.slice(0, 10)}</td>
      )
  }
}

/**
 * The slices worth keeping, and the way to keep one.
 *
 * A row of chips above the table rather than an entry in the app's left rail.
 * A view is a question about *this* screen, and the rail is where the screens
 * themselves live - a list of catalogue slices sitting there while the calendar
 * is open would be naming something not on show. Beside the controls it
 * changes, it explains itself.
 *
 * Saving is deliberately a two-step: the button opens a name field rather than
 * storing "View 3". A view nobody can tell apart from the next one is a row to
 * scroll past, which is the failure this is meant to prevent.
 */
function ViewBar({
  views,
  shape,
  onOpen,
  onSave,
  onRemove,
}: {
  views: SavedView[]
  shape: ViewShape
  onOpen: (view: SavedView) => void
  onSave: (name: string) => void
  onRemove: (id: string) => void
}) {
  const { t } = useTranslation()
  const [naming, setNaming] = useState(false)
  const [name, setName] = useState('')

  const active = views.find((view) => matchesView(view, shape))

  const commit = () => {
    const trimmed = name.trim()
    if (trimmed === '') return
    onSave(trimmed)
    setName('')
    setNaming(false)
  }

  // Nothing saved and nothing set: an empty bar with one button on it teaches
  // nobody what a view is, and takes a line of the screen to do it.
  if (views.length === 0 && !naming && !isNarrowed(shape.filter)) return null

  return (
    <div className="flex flex-wrap items-center gap-2">
      <span className="text-[11px] font-medium uppercase tracking-[0.09em] text-faint">
        {t('catalogue.views')}
      </span>

      {views.map((view) => {
        const open = active?.id === view.id
        return (
          <span
            key={view.id}
            className={cn(
              'group inline-flex items-center gap-1 rounded-full border pl-2.5 pr-1 py-0.5 text-[11.5px] transition-colors',
              open
                ? 'border-transparent bg-accent-soft font-semibold text-accent-2'
                : 'border-line text-dim hover:border-line-2 hover:text-text',
            )}
          >
            <button
              type="button"
              onClick={() => onOpen(view)}
              aria-pressed={open}
              className="cursor-pointer"
            >
              {view.name}
            </button>
            <button
              type="button"
              onClick={() => onRemove(view.id)}
              aria-label={t('catalogue.viewRemove', { name: view.name })}
              title={t('catalogue.viewRemove', { name: view.name })}
              // Always there rather than on hover: a control that appears only
              // under the pointer cannot be reached by a keyboard at all.
              className="cursor-pointer rounded-full p-0.5 text-faint transition-colors hover:bg-soft hover:text-bad"
            >
              <X className="size-3" aria-hidden />
            </button>
          </span>
        )
      })}

      {naming ? (
        <form
          className="flex items-center gap-1.5"
          onSubmit={(event) => {
            event.preventDefault()
            commit()
          }}
        >
          <Input
            autoFocus
            className="h-7 w-44 text-[12.5px]"
            value={name}
            onChange={(event) => setName(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Escape') setNaming(false)
            }}
            placeholder={t('catalogue.viewNamePlaceholder')}
            aria-label={t('catalogue.viewName')}
          />
          <Button type="submit" size="sm" variant="primary" disabled={name.trim() === ''}>
            {t('catalogue.viewSave')}
          </Button>
        </form>
      ) : (
        <Button
          size="sm"
          variant="ghost"
          onClick={() => setNaming(true)}
          // Saving the catalogue as it opens would store "everything, by score"
          // under a name, which is the one slice that needs no shortcut.
          disabled={!isNarrowed(shape.filter) && shape.groupBy === 'none'}
          title={t('catalogue.viewSaveHint')}
        >
          {active ? <Bookmark className="size-3.5" aria-hidden /> : <BookmarkPlus className="size-3.5" aria-hidden />}
          {t('catalogue.viewSaveCurrent')}
        </Button>
      )}
    </div>
  )
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

/**
 * What can be done to the chosen rows, other than ask the assistant.
 *
 * One button rather than a row of controls. A dropdown sitting among flat
 * buttons was taller than all of them and read as the most important thing in
 * the bar, which "move to status" is not - and it put deleting a batch one
 * unguarded click away from the assistant's actions.
 */
function BulkMenu({
  statuses,
  busy,
  onSetStatus,
  onUnschedule,
  onDelete,
}: {
  statuses: { key: string; label: string }[]
  busy: boolean
  onSetStatus: (status: string) => void
  onUnschedule: () => void
  onDelete: () => void
}) {
  const { t } = useTranslation()

  return (
    <Menu>
      <MenuTrigger
        render={<Button variant="ghost" size="sm" disabled={busy} />}
      >
        {t('catalogue.bulk.actions')}
        <ChevronDown className="ml-1 size-3.5" aria-hidden />
      </MenuTrigger>

      <MenuPopup align="start">
        <MenuSub>
          <MenuSubTrigger>{t('catalogue.bulk.setStatus')}</MenuSubTrigger>
          <MenuPopup align="start" side="right">
            {statuses.map((status) => (
              <MenuItem key={status.key} onClick={() => onSetStatus(status.key)}>
                {status.label}
              </MenuItem>
            ))}
          </MenuPopup>
        </MenuSub>

        <MenuItem onClick={onUnschedule}>{t('catalogue.bulk.unschedule')}</MenuItem>

        <MenuSeparator />

        {/* Apart and in the colour of something you cannot take back, though
            the trash means you can. */}
        <MenuItem tone="danger" onClick={onDelete}>
          {t('catalogue.action.delete')}
        </MenuItem>
      </MenuPopup>
    </Menu>
  )
}
