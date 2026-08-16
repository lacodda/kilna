import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Star, X } from 'lucide-react'
import {
  createVersion,
  deleteVersion,
  getVersion,
  listVersions,
  setCurrentVersion,
} from '@/lib/api'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { announceDeleted } from '@/lib/trash'
import { labelOf, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/Button'
import { Textarea } from '@/components/ui/Input'
import { Select } from '@/components/ui/Select'
import { Skeleton } from '@/components/ui/Skeleton'
import { cn } from '@/lib/utils'

interface Props {
  workId: string
}

// Versions of one role at a time: lyrics and style advance independently, and
// showing them interleaved would suggest otherwise.
export function VersionPanel({ workId }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const client = useQueryClient()
  const roles = profile.config.version_roles

  const [role, setRole] = useState(roles[0]?.key ?? '')
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [draft, setDraft] = useState('')

  const versions = useQuery({
    queryKey: keys.versions(workId),
    queryFn: () => listVersions(workId),
  })

  const summaries = (versions.data ?? []).filter((version) => version.role === role)
  // Open the newest of this role by default, so the panel is never blank; an
  // explicit choice wins until it disappears.
  const openId =
    selectedId !== null && summaries.some((v) => v.id === selectedId)
      ? selectedId
      : (summaries[0]?.id ?? null)

  const open = useQuery({
    queryKey: keys.version(openId ?? ''),
    queryFn: () => getVersion(openId!),
    enabled: openId !== null,
  })

  // Everything here changes which body is current, so the card and the list of
  // versions both need rereading.
  // A version changes the list, the work's current pointer, and the summary
  // the works list shows.
  const refreshed = [keys.versions(workId), keys.work(workId), keys.works]

  const settle = () => {
    for (const key of refreshed) void client.invalidateQueries({ queryKey: key })
  }

  const save = useMutation({
    mutationFn: (body: string) => createVersion(workId, { role, body }),
    onSuccess: (version) => {
      setDraft('')
      setSelectedId(version.id)
      settle()
    },
    onError: (cause) => say.failedTo(t('toast.versionSaveFailed'), cause),
  })

  const makeCurrent = useMutation({
    mutationFn: (versionId: string) => setCurrentVersion(workId, versionId),
    onSuccess: settle,
    onError: (cause) => say.failedTo(t('toast.versionSaveFailed'), cause),
  })

  const remove = useMutation({
    mutationFn: deleteVersion,
    onSuccess: (deletionId, versionId) => {
      if (selectedId === versionId) setSelectedId(null)
      announceDeleted({
        client,
        deletionId,
        message: t('toast.versionDeleted'),
        refresh: refreshed,
        // Reopen what was being read when it was thrown away.
        onUndone: () => setSelectedId(versionId),
      })
    },
    onError: (cause) => say.failedTo(t('toast.versionSaveFailed'), cause),
  })

  return (
    <section className="flex flex-col gap-3">
      <div className="flex items-center gap-3">
        <h3 className="text-sm font-semibold">{t('versions.title')}</h3>
        {roles.length > 1 && (
          <Select
            className="w-40"
            aria-label={t('versions.role')}
            value={role}
            onChange={(next) => {
              setRole(next)
              // The selection belonged to the role being left.
              setSelectedId(null)
            }}
            options={roles.map((r) => ({ value: r.key, label: r.label }))}
          />
        )}
      </div>

      <div className="grid gap-4 lg:grid-cols-[16rem_1fr]">
        <ul className="flex max-h-64 flex-col gap-1 overflow-y-auto">
          {versions.isPending && <Skeleton className="h-14 w-full" />}
          {!versions.isPending && summaries.length === 0 && (
            <li className="py-4 text-sm text-dim">{t('versions.none')}</li>
          )}
          {summaries.map((version) => (
            <li key={version.id} className="flex items-center gap-1">
              <button
                type="button"
                onClick={() => setSelectedId(version.id)}
                className={cn(
                  'flex-1 rounded-[9px] px-2 py-1.5 text-left text-sm transition-colors',
                  version.id === openId ? 'bg-accent-soft text-accent-2' : 'hover:bg-soft',
                )}
              >
                <span className="font-medium">
                  {t('versions.revision', { number: version.revision })}
                </span>
                {version.is_current && (
                  <span className="ml-1.5 rounded bg-accent-soft px-1 text-[10px] uppercase tracking-wide">
                    {t('versions.current')}
                  </span>
                )}
                <span className="block text-xs text-dim">
                  {t('versions.length', { count: version.length })}
                </span>
              </button>
              {!version.is_current && (
                <Button
                  variant="icon"
                  size="iconSm"
                  onClick={() => makeCurrent.mutate(version.id)}
                  title={t('versions.makeCurrent')}
                >
                  <Star aria-hidden className="size-4" />
                </Button>
              )}
              <Button
                variant="danger"
                size="iconSm"
                onClick={() => remove.mutate(version.id)}
                title={t('versions.delete')}
              >
                <X aria-hidden className="size-3.5" />
              </Button>
            </li>
          ))}
        </ul>

        <div className="flex flex-col gap-3">
          {open.isPending && openId !== null && <Skeleton className="h-32 w-full" />}
          {open.data != null && (
            <article className="rounded-xl border border-line p-3">
              <header className="mb-2 text-xs text-dim">
                {labelOf(roles, open.data.role)} ·{' '}
                {t('versions.revision', { number: open.data.revision })} ·{' '}
                {open.data.created_at.slice(0, 10)}
              </header>
              <pre className="max-h-64 overflow-auto whitespace-pre-wrap font-mono text-sm">
                {open.data.body}
              </pre>
            </article>
          )}

          <form
            className="flex flex-col gap-2"
            onSubmit={(event) => {
              event.preventDefault()
              if (draft.trim() !== '') save.mutate(draft)
            }}
          >
            <Textarea
              rows={6}
              value={draft}
              onChange={(event) => setDraft(event.target.value)}
              placeholder={t('versions.draftPlaceholder')}
              aria-label={t('versions.draftPlaceholder')}
            />
            <div className="flex justify-end">
              <Button
                type="submit"
                variant="primary"
                disabled={draft.trim() === '' || save.isPending}
              >
                {t('versions.save')}
              </Button>
            </div>
          </form>
        </div>
      </div>
    </section>
  )
}
