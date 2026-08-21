import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { catalogue as fetchCatalogue, createWork, type ScoredWork } from '@/lib/api'
import { isNarrowed, narrow, type CatalogueFilter } from '@/lib/catalogue'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { labelOf, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/Button'
import { EmptyState } from '@/components/ui/EmptyState'
import { Input } from '@/components/ui/Input'
import { Select } from '@/components/ui/Select'
import { SkeletonList } from '@/components/ui/Skeleton'
import { cn } from '@/lib/utils'

interface Props {
  onSelect: (workId: string) => void
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
  const [filter, setFilter] = useState<CatalogueFilter>({})
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

      <div className="flex flex-wrap gap-2">
        <Input
          className="max-w-64"
          value={filter.search ?? ''}
          onChange={(event) =>
            setFilter((current) => ({ ...current, search: event.target.value || undefined }))
          }
          placeholder={t('works.search')}
          aria-label={t('works.search')}
        />
        <Select
          className="w-44"
          aria-label={t('works.status')}
          value={filter.status ?? ''}
          onChange={(value) => setFilter((current) => ({ ...current, status: value || undefined }))}
          placeholder={t('works.anyStatus')}
          options={profile.config.statuses.map((s) => ({ value: s.key, label: s.label }))}
        />
        <Select
          className="w-44"
          aria-label={t('works.kind')}
          value={filter.kind ?? ''}
          onChange={(value) => setFilter((current) => ({ ...current, kind: value || undefined }))}
          placeholder={t('works.anyKind')}
          options={profile.config.work_kinds.map((k) => ({ value: k.key, label: k.label }))}
        />
      </div>

      <Rows
        rows={rows.data}
        pending={rows.isPending}
        failed={rows.isError}
        filter={filter}
        onClearFilters={() => setFilter({})}
        onSelect={onSelect}
      />
    </div>
  )
}

function Rows({
  rows,
  pending,
  failed,
  filter,
  onClearFilters,
  onSelect,
}: {
  rows: ScoredWork[] | undefined
  pending: boolean
  failed: boolean
  filter: CatalogueFilter
  onClearFilters: () => void
  onSelect: (workId: string) => void
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

  const visible = narrow(rows, filter)

  if (visible.length === 0 && isNarrowed(filter)) {
    return (
      <EmptyState
        title={t('empty.worksFiltered')}
        body={t('empty.worksFilteredBody')}
        action={<Button onClick={onClearFilters}>{t('empty.clearFilters')}</Button>}
      />
    )
  }

  return (
    <table className="w-full text-sm">
      <thead>
        <tr className="border-b border-line text-left text-xs uppercase tracking-wide text-dim">
          <th className="py-2 pr-3 font-medium">{t('catalogue.work')}</th>
          <th className="py-2 pr-3 font-medium">{t('catalogue.tier')}</th>
          <th className="py-2 pr-3 text-right font-medium">{t('catalogue.total')}</th>
          <th className="py-2 font-medium">{t('catalogue.scored')}</th>
        </tr>
      </thead>
      <tbody>
        {visible.map((row) => (
          <tr
            key={row.work_id}
            className="cursor-pointer border-b border-line hover:bg-soft"
            onClick={() => onSelect(row.work_id)}
          >
            <td className="py-2 pr-3">
              <span className="font-medium">{row.title}</span>
              <span className="ml-2 text-xs text-dim">
                {labelOf(profile.config.statuses, row.status)} ·{' '}
                {labelOf(profile.config.work_kinds, row.kind)}
              </span>
            </td>
            <td className="py-2 pr-3">
              {row.tier === null ? (
                <span className="text-faint">—</span>
              ) : (
                <span className="rounded bg-accent-soft px-1.5 py-0.5 text-xs">
                  {labelOf(profile.config.tiers, row.tier)}
                </span>
              )}
            </td>
            <td
              className={cn('py-2 pr-3 text-right tabular-nums', row.total === null && 'text-faint')}
            >
              {row.total?.toFixed(1) ?? '—'}
            </td>
            <td className="py-2 text-xs text-dim">
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
          </tr>
        ))}
      </tbody>
    </table>
  )
}
