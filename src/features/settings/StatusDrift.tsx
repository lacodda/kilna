import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { StatusChange } from '@/lib/api/types'
import { resyncStatuses, statusDrift } from '@/lib/api/works'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { allOf, say as sayLabel, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { FieldGroup } from '@/components/ui/field'

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
  // `null` is "not looked yet", an empty array is "looked, nothing to do" —
  // two different things that must not read the same on screen.
  const [found, setFound] = useState<StatusChange[] | null>(null)

  const label = (key: string) => {
    const found = allOf(profile.config, 'statuses').find((status) => status.key === key)?.label
    return found === undefined ? key : sayLabel(found)
  }

  const check = useAppMutation({
    mutationFn: statusDrift,
    onSuccess: setFound,
    failure: 'toast.statusDriftFailed',
  })

  const apply = useAppMutation({
    mutationFn: resyncStatuses,
    refresh: refresh.work,
    onSuccess: (changes) => {
      setFound([])
      say.ok(t('data.statusResynced', { count: changes.length }))
    },
    failure: 'toast.statusResyncFailed',
  })

  return (
    <section className="flex max-w-3xl flex-col gap-2">
      {/* The explanation under the buttons it explains, and the findings
          under both: they arrive only once asked for, and a hint pushed below
          a list of forty works would be a hint nobody reaches. */}
      <FieldGroup label={t('data.statusTitle')} help={t('data.statusHint')}>
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
      </FieldGroup>

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
