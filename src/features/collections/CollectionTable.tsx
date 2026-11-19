import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { Collection, MetaField, ScoredWork, Work, WorkPlan } from '@/lib/api/types'
import { updateWork } from '@/lib/api/works'
import { formatWeekday } from '@/lib/format'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { Select } from '@/components/AppSelect'
import { InlineField, numberCodec } from '@/components/ui/inline-field'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  TableScroll,
} from '@/components/ui/table'

/** The kinds of field a cell of the table can hold: one line, a number, a
 *  choice. A longer text is written on the card. */
const CELLS: readonly MetaField['type'][] = ['text', 'number', 'choice']

interface Props {
  collection: Collection
  /** The collection's works as the catalogue reads them: their kinds say
   *  which fields are columns. */
  works: readonly ScoredWork[]
  /** The works planned on a day: a draft's day is its slot's. */
  plans: readonly WorkPlan[]
}

/**
 * A month's drafts as a table (v0.152, FR-20 20.3): a row a work, its day,
 * its title and every one-line field its kind has - a genre, a tradition, a
 * tempo, a mood, what its cover shows - each written in place and kept on
 * leaving the cell, an undo a cell. Thirteen tracks of a month filled in
 * one sitting rather than thirteen cards opened.
 */
export function CollectionTable({ collection, works, plans }: Props) {
  const { t } = useTranslation()
  const { config } = useProfile()
  const kinds = new Set(works.map((work) => work.kind))
  const columns = config.work_meta_fields.filter(
    (field) =>
      CELLS.includes(field.type) &&
      ((field.kinds ?? []).length === 0 || (field.kinds ?? []).some((kind) => kinds.has(kind))),
  )
  // The rows by their day, the undated after, in the collection's order.
  const dayOf = (id: string) => plans.find((plan) => plan.work_id === id)?.scheduled_at ?? null
  const ids = [...collection.work_ids].sort((a, b) => {
    const left = dayOf(a)
    const right = dayOf(b)
    if (left === right) return 0
    if (left === null) return 1
    if (right === null) return -1
    return left.localeCompare(right)
  })

  return (
    <TableScroll className="max-h-none">
      <Table aria-label={t('collections.table', { title: collection.title })}>
        <TableHead>
          <TableRow>
            <TableHeader>{t('collections.tableDay')}</TableHeader>
            <TableHeader>{t('collections.tableTitle')}</TableHeader>
            {columns.map((field) => (
              <TableHeader key={field.key}>{sayLabel(field.label)}</TableHeader>
            ))}
          </TableRow>
        </TableHead>
        <TableBody>
          {ids.map((id) => (
            <Row key={id} id={id} day={dayOf(id)} columns={columns} />
          ))}
        </TableBody>
      </Table>
    </TableScroll>
  )
}

function Row({ id, day, columns }: { id: string; day: string | null; columns: MetaField[] }) {
  const { i18n } = useTranslation()
  const work = useQuery(queries.work(id))
  const patch = useAppMutation({
    mutationFn: (change: Parameters<typeof updateWork>[1]) => updateWork(id, change),
    failure: 'toast.workSaveFailed',
    refresh: refresh.work,
  })
  const data = work.data
  if (data === undefined || data === null) return null
  return (
    <TableRow>
      <TableCell className="font-mono text-xs whitespace-nowrap text-dim">
        {day === null ? '—' : formatWeekday(day, i18n.language)}
      </TableCell>
      <TableCell className="min-w-48">
        <InlineField
          label={data.title}
          labelHidden
          value={data.title}
          onCommit={(title) => {
            if (title === null || title.trim() === '') return false
            patch.mutate({ title: title.trim() })
          }}
        />
      </TableCell>
      {columns.map((field) => (
        <TableCell key={field.key} className="min-w-36">
          <Cell
            work={data}
            field={field}
            onChange={(value) => patch.mutate({ meta: { [field.key]: value } })}
          />
        </TableCell>
      ))}
    </TableRow>
  )
}

/** One field of one work, written in place. A field its kind has not got is
 *  an empty cell, not a box. */
function Cell({
  work,
  field,
  onChange,
}: {
  work: Work
  field: MetaField
  onChange: (value: string | number | null) => void
}) {
  const label = `${sayLabel(field.label)} · ${work.title}`
  if ((field.kinds ?? []).length > 0 && !(field.kinds ?? []).includes(work.kind)) return null
  const stored = work.meta[field.key]
  switch (field.type) {
    case 'choice':
      return (
        <Select
          value={typeof stored === 'string' ? stored : ''}
          onChange={(next) => onChange(next === '' ? null : next)}
          options={(field.options ?? []).map((option) => ({
            value: option.key,
            label: sayLabel(option.label),
          }))}
          aria-label={label}
          className="w-full"
        />
      )
    case 'number':
      return (
        <InlineField
          label={label}
          labelHidden
          placeholder="—"
          value={typeof stored === 'number' ? stored : null}
          codec={numberCodec}
          onCommit={(value) => onChange(value)}
        />
      )
    default:
      return (
        <InlineField
          label={label}
          labelHidden
          placeholder="—"
          value={typeof stored === 'string' ? stored : null}
          onCommit={(value) => onChange(value === null ? null : value.trim())}
        />
      )
  }
}
