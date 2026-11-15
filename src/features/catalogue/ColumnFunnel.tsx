import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { isColumnFiltered, type ColumnFilters, type ColumnId } from '@/lib/catalogue'
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

  const shell = (body: ReactNode) => (
    <FilterPopover
      title={label}
      label={t('catalogue.filterColumn', { column: label })}
      active={isColumnFiltered(filters, narrows)}
      clearLabel={t('catalogue.clear')}
      onClear={() => onChange({ ...filters, [narrows]: undefined })}
      // At rest the funnel is out of the way; it stays while its column is
      // narrowing, or a table narrowed by a funnel would look like a
      // smaller table.
      className={cn(
        'opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100',
        'data-[active]:opacity-100 data-[popup-open]:opacity-100',
      )}
    >
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
          onChange={(statuses) => onChange({ ...filters, statuses })}
        />,
      )
    case 'stages':
      return shell(
        <StageChecks
          chosen={filters.stages ?? []}
          onChange={(stages) => onChange({ ...filters, stages })}
        />,
      )
    case 'tiers':
      return shell(
        <CheckList
          options={allOf(profile.config, 'tiers').map((tier) => ({
            value: tier.key,
            label: sayLabel(tier.label),
          }))}
          chosen={filters.tiers ?? []}
          onChange={(tiers) => onChange({ ...filters, tiers })}
        />,
      )
    case 'marks': {
      // A profile that defines no marks has nothing to tick, and a funnel
      // opening on an empty panel would be a control that does nothing.
      const marks = profile.config.marks ?? []
      if (marks.length === 0) return null
      return shell(
        <CheckList
          options={marks.map((mark) => ({ value: mark.key, label: sayLabel(mark.label) }))}
          chosen={filters.marks ?? []}
          onChange={(next) => onChange({ ...filters, marks: next })}
        />,
      )
    }
  }
}
