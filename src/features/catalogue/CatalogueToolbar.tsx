import { useTranslation } from 'react-i18next'
import { Star } from 'lucide-react'
import { GAPS, type CatalogueFilter, type ColumnId, type Gap, type GroupBy } from '@/lib/catalogue'
import type { SavedView, ViewShape } from '@/lib/views'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { ColumnPicker } from './ColumnPicker'
import { GroupMenu } from './GroupMenu'
import { QueryBox } from './QueryBox'
import { ViewsMenu } from './ViewsMenu'

interface Props {
  filter: CatalogueFilter
  /** The kind chips and the star write the filter this way, so the box follows. */
  onControl: (change: Partial<CatalogueFilter>) => void
  /** The gap chips write it this way: they are not part of the line. */
  onGap: (gap: Gap | undefined) => void
  query: string
  onQuery: (line: string) => void
  unknown: { field: string; value: string } | undefined
  views: SavedView[]
  shape: ViewShape
  onOpenView: (view: SavedView) => void
  onSaveView: (name: string) => void
  onRemoveView: (id: string) => void
  columns: ColumnId[]
  onColumns: (next: ColumnId[]) => void
  onMoveColumn: (id: ColumnId, to: number) => void
  groupBy: GroupBy
  onGroupBy: (next: GroupBy) => void
  /** Every work, by kind, and every gap: the chips count the whole catalogue. */
  kindCounts: ReadonlyMap<string, number>
  gapCounts: Record<Gap, number> | undefined
  /** What the table shows, and of how many; undefined while it loads. */
  shown: number | undefined
  total: number | undefined
  narrowed: boolean
  chosen: number
  onClearFilters: () => void
}

/**
 * What stands over the table: two rows, as the mockup draws them.
 *
 * The first asks - the query box, then the buttons that shape the table:
 * views, columns, grouping, the star. The second switches - the kind the
 * catalogue is in, the gaps worth closing - and ends in the count. Until v0.79
 * there were up to seven rows here, with a dropdown for the status and one
 * for the tier beside a query box that says `status:` and `tier:` itself and
 * column funnels that tick either; the dropdowns went, the box and the funnels
 * stayed, and views and grouping moved into the first row's buttons.
 */
export function CatalogueToolbar(props: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const { filter } = props

  return (
    <>
      <div className="flex w-full flex-wrap items-center gap-1.5">
        <QueryBox
          value={props.query}
          onChange={props.onQuery}
          unknown={props.unknown}
          shown={props.narrowed ? props.shown : undefined}
          total={props.total ?? 0}
        />
        <ViewsMenu
          views={props.views}
          shape={props.shape}
          onOpen={props.onOpenView}
          onSave={props.onSaveView}
          onRemove={props.onRemoveView}
        />
        <ColumnPicker
          columns={props.columns}
          onChange={props.onColumns}
          onMove={props.onMoveColumn}
        />
        <GroupMenu value={props.groupBy} onChange={props.onGroupBy} />
        {/* The works marked to come back to. Not a token in the box - a star is
            raised and lowered with a click, and is asked for the same way. A
            button in the row of buttons rather than a chip among the kinds:
            it is a narrowing you switch, like grouping, and on it wears the
            soft accent of something chosen. The star keeps the warn colour it
            wears on every row and on the card, because the star is what it
            filters by. */}
        <Button
          size="sm"
          variant={filter.bookmarked === true ? 'soft' : 'ghost'}
          aria-pressed={filter.bookmarked === true}
          title={t('catalogue.starredHint')}
          onClick={() =>
            props.onControl({ bookmarked: filter.bookmarked === true ? undefined : true })
          }
        >
          <Star
            aria-hidden
            className={filter.bookmarked === true ? 'fill-current text-warn' : undefined}
          />
          {t('catalogue.starred')}
        </Button>
      </div>

      <div className="flex w-full flex-wrap items-center gap-1.5">
        {/* The kind of work as a row of chips, not one more dropdown: it is the
            mode the catalogue is in — songs, videos — and a mode is read at a
            glance and switched in one click. Hidden while the profile has one
            kind: "all" beside the only thing there is would be a choice of one.
            The counts are of the whole catalogue, so a kind reads as empty
            rather than as absent.

            One at a time, and the chip that is on turns off: the group then
            holds nothing, which is every kind again, without a separate
            control for it. */}
        {profile.config.work_kinds.length > 1 && (
          <ChipGroup
            aria-label={t('works.kind')}
            value={[filter.kind ?? ANY_KIND]}
            onValueChange={(next) => {
              const picked = next[0]
              props.onControl({
                kind: picked === undefined || picked === ANY_KIND ? undefined : picked,
              })
            }}
          >
            <Chip value={ANY_KIND} count={props.total}>
              {t('catalogue.kindAll')}
            </Chip>
            {profile.config.work_kinds.map((kind) => (
              <Chip key={kind.key} value={kind.key} count={props.kindCounts.get(kind.key) ?? 0}>
                {sayLabel(kind.label)}
              </Chip>
            ))}
          </ChipGroup>
        )}

        {/* The gaps, dashed: each is a job not yet done rather than a kind of
            thing, and the count says how big the job is. Chips carry their
            words, not just an icon - the predecessor tried icons alone and
            nobody could tell which filter was on. Clicking the chip that is
            already on turns it off: one gap at a time, and no separate way to
            undo it. */}
        <ChipGroup
          aria-label={t('catalogue.gaps')}
          className="ml-2"
          value={filter.gap === undefined ? [] : [filter.gap]}
          onValueChange={(next) => props.onGap(GAPS.find((gap) => gap === next[0]))}
        >
          {GAPS.map((gap) => (
            <Chip
              key={gap}
              value={gap}
              variant="dashed"
              count={props.gapCounts?.[gap]}
              title={t(`catalogue.gapHint.${gap}`)}
            >
              {t(`catalogue.gap.${gap}`)}
            </Chip>
          ))}
        </ChipGroup>

        <Tally {...props} />
      </div>
    </>
  )
}

/**
 * The end of the second row: how much of the catalogue is on show and how
 * much of it is ticked - or, while the box holds a value the profile does not
 * have, which one, since that is why the count would be wrong.
 */
function Tally({ unknown, shown, total, narrowed, chosen, onClearFilters }: Props) {
  const { t } = useTranslation()
  if (total === undefined || shown === undefined) return null

  if (unknown !== undefined) {
    return (
      <span className="ml-auto text-xs text-bad" role="status">
        {t('catalogue.queryUnknown', { field: unknown.field, value: unknown.value })}
      </span>
    )
  }

  // "3 of 3" says nothing; the whole count does until something narrows it,
  // and then the number that matters is how much is out of sight.
  const count = narrowed
    ? t('catalogue.showing', { shown, total })
    : t('catalogue.count', { count: total })
  return (
    <span className="ml-auto flex items-center gap-2 font-mono text-xs text-faint">
      <span>
        {count}
        {chosen > 0 && ` · ${t('catalogue.chosen', { count: chosen })}`}
      </span>
      {narrowed && (
        <Button variant="link" className="font-sans" onClick={onClearFilters}>
          {t('catalogue.clear')}
        </Button>
      )}
    </span>
  )
}

/** The chip standing for every kind. A chip in a group needs a value, and Base
 * UI swaps an empty string for a generated id; the leading space keeps it from
 * ever meeting a profile's own kind key. */
const ANY_KIND = ' any'
