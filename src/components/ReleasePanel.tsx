import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { X } from 'lucide-react'
import { createRelease, deleteRelease, releasesForWork } from '@/lib/api'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { announceDeleted } from '@/lib/trash'
import { labelOf, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/Button'
import { Select } from '@/components/ui/Select'
import { Skeleton } from '@/components/ui/Skeleton'

interface Props {
  workId: string
  workTitle: string
}

// Where a work enters the release queue. Scheduling happens in the calendar;
// here it is only "this work will go out as a clip".
export function ReleasePanel({ workId, workTitle }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const client = useQueryClient()
  const kinds = profile.config.release_kinds

  const [kind, setKind] = useState(kinds[0]?.key ?? '')

  const releases = useQuery({
    queryKey: keys.releasesForWork(workId),
    queryFn: () => releasesForWork(workId),
  })

  // What a release touches, whether it is added, removed or brought back — the
  // calendar, the queue, and the line the journal keeps about it.
  const refreshed = [
    keys.journal,
    keys.releasesForWork(workId),
    keys.releases,
    keys.calendar,
    keys.releaseQueue,
  ]

  const settle = () => {
    for (const key of refreshed) void client.invalidateQueries({ queryKey: key })
  }

  const add = useMutation({
    mutationFn: () => createRelease({ work_id: workId, kind, title: workTitle }),
    onSuccess: () => {
      settle()
      say.ok(t('toast.releaseCreated'))
    },
    onError: (cause) => say.failedTo(t('toast.releaseSaveFailed'), cause),
  })

  const remove = useMutation({
    mutationFn: deleteRelease,
    onSuccess: (deletionId) =>
      announceDeleted({
        client,
        deletionId,
        message: t('toast.releaseDeleted'),
        refresh: refreshed,
      }),
    onError: (cause) => say.failedTo(t('toast.releaseSaveFailed'), cause),
  })

  const releasesData = releases.data ?? []

  return (
    <section className="flex flex-col gap-3">
      <h3 className="text-sm font-semibold">{t('releases.title')}</h3>

      {releases.isError && (
        <p role="alert" className="text-sm text-bad">
          {t('toast.loadFailed')}
        </p>
      )}

      {releases.isPending && <Skeleton className="h-14 w-full" />}

      {releasesData.length > 0 && (
        <ul className="flex flex-col gap-1">
          {releasesData.map((release) => (
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
                onClick={() => remove.mutate(release.id)}
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
        <Button onClick={() => add.mutate()} disabled={kind === '' || add.isPending}>
          {t('releases.add')}
        </Button>
      </div>
    </section>
  )
}
