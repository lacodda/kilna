import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { X } from 'lucide-react'
import { createRelease, deleteRelease, releasesForWork, type Release } from '@/lib/api'
import { labelOf, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/Button'
import { Select } from '@/components/ui/Select'

interface Props {
  workId: string
  workTitle: string
  onChanged: () => void
}

// Where a work enters the release queue. Scheduling happens in the calendar;
// here it is only "this work will go out as a clip".
export function ReleasePanel({ workId, workTitle, onChanged }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const kinds = profile.config.release_kinds

  const [releases, setReleases] = useState<Release[]>([])
  const [kind, setKind] = useState(kinds[0]?.key ?? '')
  const [error, setError] = useState<string | null>(null)
  const [revision, setRevision] = useState(0)

  useEffect(() => {
    releasesForWork(workId)
      .then((result) => {
        setReleases(result)
        setError(null)
      })
      .catch((cause: unknown) => setError(String(cause)))
  }, [workId, revision])

  const add = () => {
    createRelease({ work_id: workId, kind, title: workTitle })
      .then(() => {
        setRevision((current) => current + 1)
        onChanged()
      })
      .catch((cause: unknown) => setError(String(cause)))
  }

  return (
    <section className="flex flex-col gap-3">
      <h3 className="text-sm font-semibold">{t('releases.title')}</h3>

      {error !== null && (
        <p role="alert" className="text-sm text-bad">
          {error}
        </p>
      )}

      {releases.length > 0 && (
        <ul className="flex flex-col gap-1">
          {releases.map((release) => (
            <li
              key={release.id}
              className="flex items-center gap-3 rounded-xl border border-line px-3 py-1.5 text-sm"
            >
              <span className="font-medium">{labelOf(kinds, release.kind)}</span>
              <span className="text-xs text-dim">
                {release.released_at !== null
                  ? t('releases.releasedOn', { date: release.released_at.slice(0, 10) })
                  : (release.scheduled_at ?? t('releases.unscheduled'))}
              </span>
              {release.url !== null && (
                <a
                  href={release.url}
                  target="_blank"
                  rel="noreferrer"
                  className="truncate text-xs underline"
                >
                  {release.url}
                </a>
              )}
              <Button
                variant="danger"
                size="iconSm"
                className="ml-auto"
                title={t('releases.delete')}
                onClick={() => {
                  deleteRelease(release.id)
                    .then(() => {
                      setRevision((current) => current + 1)
                      onChanged()
                    })
                    .catch((cause: unknown) => setError(String(cause)))
                }}
              >
                <X aria-hidden className="size-3.5" />
              </Button>
            </li>
          ))}
        </ul>
      )}

      <div className="flex gap-2">
        <Select
          className="w-48"
          aria-label={t('releases.kind')}
          value={kind}
          onChange={setKind}
          options={kinds.map((k) => ({ value: k.key, label: k.label }))}
        />
        <Button onClick={add} disabled={kind === ''}>
          {t('releases.add')}
        </Button>
      </div>
    </section>
  )
}
