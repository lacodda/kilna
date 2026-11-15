import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { CalendarX } from 'lucide-react'
import { markReleasedBatch, unscheduleWorks } from '@/lib/api/releases'
import type { Slot } from '@/lib/dashboard'
import { keys } from '@/lib/query/keys'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { ConfirmAction } from '@/components/ConfirmAction'
import { Button } from '@/components/ui/button'
import { Panel, SectionLabel } from '@/components/ui/panel'

/** Everything a plan leaving the calendar, or going out, is read by. */
const REFRESHED = [
  ...refresh.works,
  keys.calendar,
  keys.releaseQueue,
  keys.releases,
  keys.register,
  keys.journal,
]

/**
 * The plans whose day has passed, answered together (v0.111).
 *
 * A planned release dated in the past is one of two things: it went out and
 * nobody marked it, or it did not go out. After the owner's shorts were
 * carried over from the predecessor there were fifty-eight of them, each a
 * card on this board to open and answer by hand - and the answer was the
 * same for all of them. So the two answers stand here as two buttons, each
 * asked about once before it is done: neither is taken back in one step.
 *
 * The one place on this screen that changes the work. Everything else on it
 * is a way to where a thing is done; this is done here because there is no
 * one place it is done - it is fifty-eight places.
 */
export function Overdue({ slots }: { slots: readonly Slot[] }) {
  const { t } = useTranslation()
  const [asking, setAsking] = useState<'release' | 'unschedule' | null>(null)
  const count = slots.length

  const release = useAppMutation({
    mutationFn: () => markReleasedBatch(slots.map((slot) => slot.release.id)),
    failure: 'toast.releaseSaveFailed',
    refresh: REFRESHED,
    onSuccess: (outcome) => {
      setAsking(null)
      say.ok(t('dashboard.overdueReleased', { count: outcome.changed }))
      say.skipped(outcome.skipped)
    },
  })

  const unschedule = useAppMutation({
    mutationFn: () => unscheduleWorks([...new Set(slots.map((slot) => slot.release.work_id))]),
    failure: 'toast.releaseSaveFailed',
    refresh: REFRESHED,
    onSuccess: (outcome) => {
      setAsking(null)
      say.ok(t('catalogue.bulk.unscheduled', { count: outcome.changed }))
      say.skipped(outcome.skipped)
    },
  })

  const busy = release.isPending || unschedule.isPending

  return (
    <section className="flex flex-col gap-2">
      <SectionLabel>
        <CalendarX aria-hidden className="size-3.5" />
        {t('dashboard.overdue')}
        <span className="font-mono text-xs tracking-normal normal-case text-dim">
          {t('dashboard.weekCount', { count })}
        </span>
      </SectionLabel>
      <Panel className="flex flex-wrap items-center gap-x-3 gap-y-2 p-3">
        <p className="min-w-0 flex-1 basis-64 text-sm text-dim">
          {t('dashboard.overdueBody', { count })}
        </p>
        <div className="flex shrink-0 gap-1.5">
          <Button size="sm" variant="soft" disabled={busy} onClick={() => setAsking('release')}>
            {t('dashboard.overdueRelease')}
          </Button>
          <Button size="sm" variant="ghost" disabled={busy} onClick={() => setAsking('unschedule')}>
            {t('dashboard.overdueUnschedule')}
          </Button>
        </div>
      </Panel>

      <ConfirmAction
        open={asking === 'release'}
        onOpenChange={(open) => {
          if (!open) setAsking(null)
        }}
        title={t('dashboard.overdueReleaseTitle', { count })}
        description={t('dashboard.overdueReleaseBody')}
        actionLabel={t('dashboard.overdueRelease')}
        pending={release.isPending}
        onConfirm={() => release.mutate()}
      />
      <ConfirmAction
        open={asking === 'unschedule'}
        onOpenChange={(open) => {
          if (!open) setAsking(null)
        }}
        title={t('dashboard.overdueUnscheduleTitle', { count })}
        description={t('dashboard.overdueUnscheduleBody')}
        actionLabel={t('dashboard.overdueUnschedule')}
        pending={unschedule.isPending}
        onConfirm={() => unschedule.mutate()}
      />
    </section>
  )
}
