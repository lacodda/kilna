import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { resyncStatuses, statusDrift, type StatusChange } from '@/lib/api'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/Button'
import { Badge } from '@/components/ui/Badge'

/**
 * Bringing every status back in line with the facts — shown before it happens.
 *
 * A mass restate is the one operation here with no undo behind it: the trash
 * holds deleted things, not overwritten fields. So the dry run is not a
 * convenience, it is the gate. You see the list, then you decide.
 */
export function StatusDrift() {
  const { t } = useTranslation()
  const profile = useProfile()
  const client = useQueryClient()
  // `null` is "not looked yet", an empty array is "looked, nothing to do" —
  // two different things that must not read the same on screen.
  const [found, setFound] = useState<StatusChange[] | null>(null)

  const label = (key: string) =>
    profile.config.statuses.find((status) => status.key === key)?.label ?? key

  const check = useMutation({
    mutationFn: statusDrift,
    onSuccess: setFound,
    onError: (cause) => say.failedTo(t('toast.statusDriftFailed'), cause),
  })

  const apply = useMutation({
    mutationFn: resyncStatuses,
    onSuccess: (changes) => {
      setFound([])
      void client.invalidateQueries({ queryKey: keys.works })
      void client.invalidateQueries({ queryKey: keys.catalogue })
      void client.invalidateQueries({ queryKey: keys.journal })
      say.ok(t('data.statusResynced', { count: changes.length }))
    },
    onError: (cause) => say.failedTo(t('toast.statusResyncFailed'), cause),
  })

  return (
    <section className="flex flex-col gap-2">
      <h3 className="text-sm font-semibold">{t('data.statusTitle')}</h3>
      <p className="text-sm text-dim">{t('data.statusHint')}</p>

      <div className="flex items-center gap-2">
        <Button onClick={() => check.mutate()} disabled={check.isPending}>
          {t('data.statusCheck')}
        </Button>
        {found != null && found.length > 0 && (
          <Button variant="primary" onClick={() => apply.mutate()} disabled={apply.isPending}>
            {t('data.statusApply', { count: found.length })}
          </Button>
        )}
      </div>

      {found != null && found.length === 0 && (
        <p className="text-sm text-dim">{t('data.statusInStep')}</p>
      )}

      {found != null && found.length > 0 && (
        <ul className="flex flex-col gap-1.5 rounded-xl border border-line p-3">
          {found.map((change) => (
            <li key={change.work_id} className="flex items-center gap-2 text-sm">
              <span className="min-w-0 flex-1 truncate">{change.title}</span>
              <Badge variant="soft">{label(change.from)}</Badge>
              <span className="text-faint">→</span>
              <Badge variant="accent">{label(change.to)}</Badge>
            </li>
          ))}
        </ul>
      )}
    </section>
  )
}
