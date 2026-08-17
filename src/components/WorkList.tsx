import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { createWork, listWorks, type WorkFilter } from '@/lib/api'
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
  selectedId: string | null
  onSelect: (id: string) => void
}

export function WorkList({ selectedId, onSelect }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const client = useQueryClient()
  const [filter, setFilter] = useState<WorkFilter>({})
  const [title, setTitle] = useState('')

  const works = useQuery({
    queryKey: keys.workList(filter),
    queryFn: () => listWorks(filter),
    // Typing in the search box changes the key on every keystroke; without this
    // the list would blink back to a skeleton between each one.
    placeholderData: (previous) => previous,
  })

  const add = useMutation({
    mutationFn: createWork,
    onSuccess: (work) => {
      setTitle('')
      // The new work has to exist in the list before we can select it.
      void client.invalidateQueries({ queryKey: keys.works })
      void client.invalidateQueries({ queryKey: keys.workspace })
      void client.invalidateQueries({ queryKey: keys.journal })
      say.ok(t('toast.workCreated'))
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

  const isFiltered =
    filter.search !== undefined || filter.status !== undefined || filter.kind !== undefined

  return (
    <div className="flex h-full flex-col gap-3">
      <form
        className="flex gap-2"
        onSubmit={(event) => {
          event.preventDefault()
          submit()
        }}
      >
        <Input
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
          className="max-w-48"
          value={filter.search ?? ''}
          onChange={(event) =>
            setFilter((current) => ({ ...current, search: event.target.value || undefined }))
          }
          placeholder={t('works.search')}
          aria-label={t('works.search')}
        />
        <Select
          className="w-40"
          aria-label={t('works.status')}
          value={filter.status ?? ''}
          onChange={(value) => setFilter((current) => ({ ...current, status: value || undefined }))}
          placeholder={t('works.anyStatus')}
          options={profile.config.statuses.map((s) => ({ value: s.key, label: s.label }))}
        />
        <Select
          className="w-40"
          aria-label={t('works.kind')}
          value={filter.kind ?? ''}
          onChange={(value) => setFilter((current) => ({ ...current, kind: value || undefined }))}
          placeholder={t('works.anyKind')}
          options={profile.config.work_kinds.map((k) => ({ value: k.key, label: k.label }))}
        />
      </div>

      {works.isPending ? (
        <SkeletonList rows={6} />
      ) : works.isError ? (
        <p role="alert" className="text-sm text-bad">
          {t('toast.loadFailed')}
        </p>
      ) : works.data.length === 0 ? (
        isFiltered ? (
          <EmptyState
            className="border-0"
            title={t('empty.worksFiltered')}
            body={t('empty.worksFilteredBody')}
            action={
              <Button onClick={() => setFilter({})}>{t('empty.clearFilters')}</Button>
            }
          />
        ) : (
          <EmptyState
            className="border-0"
            title={t('empty.worksTitle')}
            body={t('empty.worksBody')}
          />
        )
      ) : (
        <ul className={cn('flex flex-col gap-1 overflow-y-auto', works.isPlaceholderData && 'opacity-60')}>
          {works.data.map((work) => (
            <li key={work.id}>
              <button
                type="button"
                onClick={() => onSelect(work.id)}
                className={cn(
                  'w-full rounded-[9px] px-3 py-2 text-left transition-colors',
                  work.id === selectedId ? 'bg-accent-soft text-accent-2' : 'hover:bg-soft',
                )}
              >
                <span className="block truncate font-medium">{work.title}</span>
                <span className="text-xs text-dim">
                  {labelOf(profile.config.statuses, work.status)} ·{' '}
                  {labelOf(profile.config.work_kinds, work.kind)}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}
