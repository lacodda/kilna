import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import {
  andBlocked,
  clearedColumn,
  combines,
  EMPTY_TICK,
  isColumnFiltered,
  logicOf,
  NO_VIBE,
  vibeTick,
  withLogic,
  withTicks,
  type ColumnFilters,
  type ColumnId,
  type Combine,
  type Polarity,
} from '@/lib/catalogue'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { axesOf } from '@/lib/vibe'
import { allOf, say as sayLabel, useProfile } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { FilterPopover } from '@/components/ui/filter-popover'
import { Input } from '@/components/ui/input'
import { CheckList } from '@/components/CheckList'
import { StageChecks } from '@/components/StageChecks'
import { COLUMN_SPECS } from './columns'

interface Props {
  column: ColumnId
  filters: ColumnFilters
  onChange: (next: ColumnFilters) => void
}

/**
 * The funnel in a column's heading: what it holds and how it is drawn, or
 * nothing for a column that answers no narrowing question.
 *
 * The words are the profile's - the same lists the query box reads. Since
 * v0.79 the status and the tier have no dropdown of their own in the toolbar:
 * a status or a tier is typed into the box (`status:draft`) or ticked here,
 * and a third way to say the same thing was one too many.
 */
export function ColumnFunnel({ column, filters, onChange }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const narrows = COLUMN_SPECS[column].filter
  if (narrows === undefined) return null
  const label = t(COLUMN_SPECS[column].label)

  // How the ticks are read (v0.139, FR-12 12.2): with them or without, any
  // of them or all - "all" closed where a work holds one value and two are
  // ticked, said on the control rather than left to guess.
  const logic = logicOf(filters, narrows)
  const held = filters[narrows]
  const ticks =
    typeof held === 'string' ? held.split(/\s+/).filter(Boolean).length : (held?.length ?? 0)
  const blocked = andBlocked(narrows, ticks)
  const switches = (
    <div className="flex flex-wrap items-center gap-2 pb-2">
      <SegmentedControl
        aria-label={t('catalogue.logic.polarity')}
        value={logic.polarity}
        onValueChange={(polarity) =>
          onChange(withLogic(filters, narrows, { polarity: polarity as Polarity }))
        }
      >
        <Segment value="include">{t('catalogue.logic.include')}</Segment>
        <Segment value="exclude">{t('catalogue.logic.exclude')}</Segment>
      </SegmentedControl>
      {combines(narrows) && (
        <span title={blocked ? t('catalogue.logic.andBlocked') : undefined}>
          <SegmentedControl
            aria-label={t('catalogue.logic.combine')}
            value={blocked ? 'or' : logic.combine}
            onValueChange={(combine) =>
              onChange(withLogic(filters, narrows, { combine: combine as Combine }))
            }
          >
            <Segment value="or">{t('catalogue.logic.or')}</Segment>
            <Segment value="and" disabled={blocked}>
              {t('catalogue.logic.and')}
            </Segment>
          </SegmentedControl>
        </span>
      )}
    </div>
  )

  const shell = (body: ReactNode) => (
    <FilterPopover
      title={label}
      label={t('catalogue.filterColumn', { column: label })}
      active={isColumnFiltered(filters, narrows)}
      clearLabel={t('catalogue.clear')}
      // Back to the start: no ticks, and with any of them.
      onClear={() => onChange(clearedColumn(filters, narrows))}
      // At rest the funnel is out of the way; it stays while its column is
      // narrowing, or a table narrowed by a funnel would look like a
      // smaller table.
      className={cn(
        'opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100',
        'data-[active]:opacity-100 data-[popup-open]:opacity-100',
      )}
    >
      {switches}
      {body}
    </FilterPopover>
  )

  switch (narrows) {
    case 'title':
      return shell(
        <Input
          autoFocus
          className="text-sm"
          value={filters.title ?? ''}
          onChange={(event) => onChange({ ...filters, title: event.target.value || undefined })}
          placeholder={t('catalogue.filterTitlePlaceholder')}
          aria-label={label}
        />,
      )
    case 'statuses':
      return shell(
        <CheckList
          options={allOf(profile.config, 'statuses').map((status) => ({
            value: status.key,
            label: sayLabel(status.label),
          }))}
          chosen={filters.statuses ?? []}
          onChange={(statuses) => onChange(withTicks(filters, 'statuses', statuses))}
        />,
      )
    case 'stages':
      return shell(
        <StageChecks
          chosen={filters.stages ?? []}
          onChange={(stages) => onChange(withTicks(filters, 'stages', stages))}
        />,
      )
    case 'tiers':
      return shell(
        <CheckList
          options={[
            { value: EMPTY_TICK, label: t('catalogue.logic.empty') },
            ...allOf(profile.config, 'tiers').map((tier) => ({
              value: tier.key,
              label: sayLabel(tier.label),
            })),
          ]}
          chosen={filters.tiers ?? []}
          onChange={(tiers) => onChange(withTicks(filters, 'tiers', tiers))}
        />,
      )
    case 'vibe': {
      // The three lines, each with its words, and the works nobody has
      // said anything of (FR-02). A profile with no lines has no funnel.
      const axes = axesOf(profile.config)
      if (axes === undefined) return null
      const lines = [
        ['mood', axes.moods],
        ['character', axes.characters],
        ['complexity', axes.complexities],
      ] as const
      return shell(
        <CheckList
          options={[
            { value: NO_VIBE, label: t('vibe.unsaid') },
            ...lines.flatMap(([line, words]) =>
              words.map((word) => ({
                value: vibeTick(line, word.key),
                label: sayLabel(word.label),
              })),
            ),
          ]}
          chosen={filters.vibe ?? []}
          onChange={(vibe) => onChange(withTicks(filters, 'vibe', vibe))}
        />,
      )
    }
    case 'marks': {
      // A profile that defines no marks has nothing to tick, and a funnel
      // opening on an empty panel would be a control that does nothing.
      const marks = profile.config.marks ?? []
      if (marks.length === 0) return null
      return shell(
        <CheckList
          options={[
            { value: EMPTY_TICK, label: t('catalogue.logic.empty') },
            ...marks.map((mark) => ({ value: mark.key, label: sayLabel(mark.label) })),
          ]}
          chosen={filters.marks ?? []}
          onChange={(next) => onChange(withTicks(filters, 'marks', next))}
        />,
      )
    }
  }
}
