import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { isColumnFiltered, NO_STAGE, type ColumnFilters, type ColumnId } from '@/lib/catalogue'
import { stagesOf } from '@/lib/stages'
import { allOf, say as sayLabel, useProfile } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Checkbox } from '@/components/ui/checkbox'
import { FilterPopover } from '@/components/ui/filter-popover'
import { Input } from '@/components/ui/input'
import { StageDial } from '@/components/StageDial'
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
        // The dial the rows are drawn with, the stop's name, and the
        // fraction it stands for.
        //
        // The name was left out when this was written, on the argument that
        // a funnel of six words beside a column of six rings asks the eye to
        // match one against the other. What it actually produced was a
        // funnel of bare percentages — a scale nobody had been told the
        // meaning of, since "60%" is not what the card, the picker or the
        // row call that stop. The dial carries the matching; the word says
        // which stop it is; the percentage stays for the person who thinks
        // in the number.
        //
        // "No stage" leads, because it is the answer to a different question
        // from the six below it — not how far along, but never said — and it
        // is the entry the owner went looking for and could not find.
        <CheckList
          options={[
            { value: NO_STAGE, label: t('stage.unset') },
            ...stagesOf(profile.config).map((stop) => ({
              value: stop.percent,
              label: t('stage.atPercent', { stage: sayLabel(stop.label), percent: stop.percent }),
              icon: (
                <StageDial percent={stop.percent} stage={stop} size={16} className="shrink-0" />
              ),
            })),
          ]}
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

/**
 * A handful of boxes to tick, for a funnel over a column of keys.
 *
 * The set's Checkbox, as the rows use: three to eight of them in a panel
 * need no list navigation, and the same box in the panel and on the row
 * reads as the same kind of control.
 */
function CheckList<T extends string | number>({
  options,
  chosen,
  onChange,
}: {
  /** `icon` is drawn before the label, for a column the rows draw as a picture
   * rather than as a word - the funnel then offers what the column shows. */
  options: { value: T; label: string; icon?: ReactNode }[]
  chosen: T[]
  onChange: (next: T[]) => void
}) {
  return (
    <>
      {options.map((option) => {
        const on = chosen.includes(option.value)
        return (
          <Checkbox
            key={String(option.value)}
            checked={on}
            onCheckedChange={() =>
              onChange(
                on ? chosen.filter((value) => value !== option.value) : [...chosen, option.value],
              )
            }
            className="px-1 py-0.5"
          >
            {option.icon}
            <span className="truncate">{option.label}</span>
          </Checkbox>
        )
      })}
    </>
  )
}
