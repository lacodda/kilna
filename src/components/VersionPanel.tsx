import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Star, X } from 'lucide-react'
import {
  createVersion,
  deleteVersion,
  getVersion,
  listVersions,
  setCurrentVersion,
  type Version,
  type VersionSummary,
} from '@/lib/api'
import { labelOf, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/Button'
import { Textarea } from '@/components/ui/Input'
import { Select } from '@/components/ui/Select'
import { cn } from '@/lib/utils'

interface Props {
  workId: string
  onChanged: () => void
}

// Versions of one role at a time: lyrics and style advance independently, and
// showing them interleaved would suggest otherwise.
export function VersionPanel({ workId, onChanged }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const roles = profile.config.version_roles

  const [role, setRole] = useState(roles[0]?.key ?? '')
  const [summaries, setSummaries] = useState<VersionSummary[]>([])
  const [openId, setOpenId] = useState<string | null>(null)
  const [open, setOpen] = useState<Version | null>(null)
  const [draft, setDraft] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [revision, setRevision] = useState(0)

  const refetch = () => setRevision((current) => current + 1)

  useEffect(() => {
    listVersions(workId)
      .then((all) => {
        const mine = all.filter((version) => version.role === role)
        setSummaries(mine)
        // Open the newest by default so the panel is never blank.
        setOpenId((current) =>
          current !== null && mine.some((v) => v.id === current) ? current : (mine[0]?.id ?? null),
        )
        setError(null)
      })
      .catch((cause: unknown) => setError(String(cause)))
  }, [workId, role, revision])

  useEffect(() => {
    // Resolving the empty case through a promise as well keeps the state update
    // asynchronous, which is what React wants from an effect.
    const pending = openId === null ? Promise.resolve(null) : getVersion(openId)

    let cancelled = false
    pending
      .then((version) => {
        if (!cancelled) setOpen(version)
      })
      .catch((cause: unknown) => {
        if (!cancelled) setError(String(cause))
      })

    return () => {
      cancelled = true
    }
  }, [openId])

  const save = () => {
    if (draft.trim() === '') return

    createVersion(workId, { role, body: draft })
      .then((version) => {
        setDraft('')
        setOpenId(version.id)
        refetch()
        onChanged()
      })
      .catch((cause: unknown) => setError(String(cause)))
  }

  const makeCurrent = (versionId: string) => {
    setCurrentVersion(workId, versionId)
      .then(() => {
        refetch()
        onChanged()
      })
      .catch((cause: unknown) => setError(String(cause)))
  }

  const remove = (versionId: string) => {
    deleteVersion(versionId)
      .then(() => {
        if (openId === versionId) setOpenId(null)
        refetch()
        onChanged()
      })
      .catch((cause: unknown) => setError(String(cause)))
  }

  return (
    <section className="flex flex-col gap-3">
      <div className="flex items-center gap-3">
        <h3 className="text-sm font-semibold">{t('versions.title')}</h3>
        {roles.length > 1 && (
          <Select
            className="w-40"
            aria-label={t('versions.role')}
            value={role}
            onChange={setRole}
            options={roles.map((r) => ({ value: r.key, label: r.label }))}
          />
        )}
      </div>

      {error !== null && (
        <p role="alert" className="text-sm text-bad">
          {error}
        </p>
      )}

      <div className="grid gap-4 lg:grid-cols-[16rem_1fr]">
        <ul className="flex max-h-64 flex-col gap-1 overflow-y-auto">
          {summaries.length === 0 && (
            <li className="py-4 text-sm text-dim">{t('versions.none')}</li>
          )}
          {summaries.map((version) => (
            <li key={version.id} className="flex items-center gap-1">
              <button
                type="button"
                onClick={() => setOpenId(version.id)}
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
                  onClick={() => makeCurrent(version.id)}
                  title={t('versions.makeCurrent')}
                >
                  <Star aria-hidden className="size-4" />
                </Button>
              )}
              <Button
                variant="danger"
                size="iconSm"
                onClick={() => remove(version.id)}
                title={t('versions.delete')}
              >
                <X aria-hidden className="size-3.5" />
              </Button>
            </li>
          ))}
        </ul>

        <div className="flex flex-col gap-3">
          {open !== null && (
            <article className="rounded-xl border border-line p-3">
              <header className="mb-2 text-xs text-dim">
                {labelOf(roles, open.role)} · {t('versions.revision', { number: open.revision })} ·{' '}
                {open.created_at.slice(0, 10)}
              </header>
              <pre className="max-h-64 overflow-auto whitespace-pre-wrap font-mono text-sm">
                {open.body}
              </pre>
            </article>
          )}

          <form
            className="flex flex-col gap-2"
            onSubmit={(event) => {
              event.preventDefault()
              save()
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
              <Button type="submit" variant="primary" disabled={draft.trim() === ''}>
                {t('versions.save')}
              </Button>
            </div>
          </form>
        </div>
      </div>
    </section>
  )
}
