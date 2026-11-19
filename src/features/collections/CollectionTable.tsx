import { useTranslation } from 'react-i18next'
import { Copy, Image } from 'lucide-react'
import { useQuery } from '@tanstack/react-query'
import type { Collection, MetaField, ScoredWork, Work, WorkPlan } from '@/lib/api/types'
import { getWork, updateWork } from '@/lib/api/works'
import { formatWeekday } from '@/lib/format'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { composedRole, coverText, roleText } from '@/lib/roleText'
import { say } from '@/lib/toast'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { Select } from '@/components/AppSelect'
import { Button } from '@/components/ui/button'
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

  // The whole month as one text to paste where the tracks are made: each
  // work's title, its prompt and its cover (v0.153, FR-20 20.4).
  const copyMonth = async () => {
    const parts: string[] = []
    for (const id of ids) {
      const work = await getWork(id)
      if (work === null) continue
      const role = composedRole(config, work.kind)
      const sound = role === null ? null : await roleText(id, role)
      const cover = coverText(work)
      parts.push(
        [
          `## ${work.title}`,
          sound === null ? null : `${t('collections.copiedSound')}: ${sound}`,
          cover === null ? null : `${t('collections.copiedCover')}: ${cover}`,
        ]
          .filter((part) => part !== null)
          .join('\n'),
      )
    }
    await copy(parts.join('\n\n'), t('collections.copiedMonth', { count: parts.length }))
  }

  return (
    <div className="flex flex-col gap-1.5">
      <Button size="xs" variant="ghost" className="self-end" onClick={() => void copyMonth()}>
        <Copy aria-hidden />
        {t('collections.copyMonth')}
      </Button>
      <TableScroll className="max-h-none">
        <Table aria-label={t('collections.table', { title: collection.title })}>
          <TableHead>
            <TableRow>
              <TableHeader>{t('collections.tableDay')}</TableHeader>
              <TableHeader>{t('collections.tableTitle')}</TableHeader>
              <TableHeader>{t('collections.tableCopy')}</TableHeader>
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
    </div>
  )
}

/** Put `text` on the clipboard and say so. */
async function copy(text: string, said: string) {
  try {
    await navigator.clipboard.writeText(text)
    say.ok(said)
  } catch (cause) {
    say.failed(cause)
  }
}

function Row({ id, day, columns }: { id: string; day: string | null; columns: MetaField[] }) {
  const { t, i18n } = useTranslation()
  const { config } = useProfile()
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
      {/* The text a generator is given, a press away (v0.153): the
          prompt its sound is written out of, and its cover's. */}
      <TableCell className="whitespace-nowrap">
        {composedRole(config, data.kind) !== null && (
          <Button
            size="icon-xs"
            variant="icon"
            aria-label={t('collections.copySound', { title: data.title })}
            title={t('collections.copySound', { title: data.title })}
            onClick={() => {
              const role = composedRole(config, data.kind)
              if (role === null) return
              void roleText(id, role).then((text) =>
                text === null
                  ? say.warn(t('collections.nothingToCopy'))
                  : copy(text, t('collections.copied')),
              )
            }}
          >
            <Copy aria-hidden />
          </Button>
        )}
        {coverText(data) !== null && (
          <Button
            size="icon-xs"
            variant="icon"
            aria-label={t('collections.copyCover', { title: data.title })}
            title={t('collections.copyCover', { title: data.title })}
            onClick={() => void copy(coverText(data) ?? '', t('collections.copied'))}
          >
            <Image aria-hidden />
          </Button>
        )}
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
