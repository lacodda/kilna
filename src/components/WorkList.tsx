import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { createWork, listWorks, type Work, type WorkFilter } from '@/lib/api'
import { labelOf, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/Button'
import { Input } from '@/components/ui/Input'
import { Select } from '@/components/ui/Select'
import { cn } from '@/lib/utils'

interface Props {
  selectedId: string | null
  onSelect: (id: string) => void
  /// Bumped by the parent when a work changed elsewhere, to force a refetch.
  revision: number
  onChanged: () => void
}

export function WorkList({ selectedId, onSelect, revision, onChanged }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const [works, setWorks] = useState<Work[]>([])
  const [filter, setFilter] = useState<WorkFilter>({})
  const [error, setError] = useState<string | null>(null)
  const [title, setTitle] = useState('')

  useEffect(() => {
    listWorks(filter)
      .then((result) => {
        setWorks(result)
        setError(null)
      })
      .catch((cause: unknown) => setError(String(cause)))
  }, [filter, revision])

  const add = () => {
    const trimmed = title.trim()
    if (trimmed === '') return

    const kind = profile.config.work_kinds[0]?.key
    if (kind === undefined) return

    createWork({ kind, title: trimmed })
      .then((work) => {
        setTitle('')
        onChanged()
        onSelect(work.id)
      })
      .catch((cause: unknown) => setError(String(cause)))
  }

  return (
    <div className="flex h-full flex-col gap-3">
      <form
        className="flex gap-2"
        onSubmit={(event) => {
          event.preventDefault()
          add()
        }}
      >
        <Input
          value={title}
          onChange={(event) => setTitle(event.target.value)}
          placeholder={t('works.newPlaceholder')}
          aria-label={t('works.newPlaceholder')}
        />
        <Button type="submit" variant="primary" disabled={title.trim() === ''}>
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

      {error !== null && (
        <p role="alert" className="text-sm text-bad">
          {error}
        </p>
      )}

      {works.length === 0 ? (
        <p className="py-8 text-center text-sm text-dim">{t('works.none')}</p>
      ) : (
        <ul className="flex flex-col gap-1 overflow-y-auto">
          {works.map((work) => (
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
