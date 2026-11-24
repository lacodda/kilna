import { useTranslation } from 'react-i18next'
import type { WorkPlan } from '@/lib/api/types'
import { deleteWorkPlan, fulfillWorkPlan, updateWorkPlan } from '@/lib/api/workPlans'
import { keys } from '@/lib/query/keys'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { hasDoors, publicationKinds, say as sayLabel, useProfile } from '@/lib/useProfile'
import { Dialog } from '@/components/AppDialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { DraftText } from '@/features/cover/Section'

/** What the window shows of a planned slot: the slot and its work. */
export interface PlanShown {
  plan: WorkPlan
  title: string
  kind: string
}

const REFRESHED = [
  keys.workPlans,
  keys.calendar,
  keys.releaseQueue,
  keys.catalogue,
  keys.works,
] as const

/**
 * A planned slot opened from the calendar (v0.138, FR-11 11.3): its work, its
 * day and note, and the one press that makes it real - an audio, a video or
 * a short made from a song with its release on the slot's day, or the work's
 * own release when its kind goes out somewhere - after which the slot is
 * gone. Taking it off the calendar leaves the work as it was.
 */
export function WorkPlanEditor({
  shown,
  onOpenChange,
  onOpenWork,
}: {
  shown: PlanShown | null
  onOpenChange: (open: boolean) => void
  onOpenWork: (workId: string) => void
}) {
  const { t } = useTranslation()
  const { config } = useProfile()
  const save = useAppMutation({
    mutationFn: ({ id, ...patch }: { id: string; scheduled_at?: string; note?: string | null }) =>
      updateWorkPlan(id, patch),
    refresh: REFRESHED,
  })
  const remove = useAppMutation({
    mutationFn: deleteWorkPlan,
    refresh: REFRESHED,
    onSuccess: () => {
      say.ok(t('workPlan.removed'))
      onOpenChange(false)
    },
  })
  const fulfill = useAppMutation({
    mutationFn: ({ id, kind }: { id: string; kind: string | null }) => fulfillWorkPlan(id, kind),
    refresh: REFRESHED,
    onSuccess: () => {
      say.ok(t('workPlan.fulfilled'))
      onOpenChange(false)
    },
  })
  if (shown === null) return <Dialog open={false} onOpenChange={onOpenChange} title="" />
  const { plan } = shown
  const ownDoor = hasDoors(config, shown.kind)
  const makeable = ownDoor ? [] : publicationKinds(config)

  return (
    <Dialog
      open
      onOpenChange={onOpenChange}
      title={shown.title}
      description={t('workPlan.description')}
    >
      <div className="flex flex-col gap-3">
        <label className="flex items-center gap-2 text-sm">
          <span className="caption w-20">{t('workPlan.day')}</span>
          <Input
            type="date"
            className="w-44"
            value={plan.scheduled_at}
            onChange={(event) => {
              if (event.target.value !== '')
                save.mutate({ id: plan.id, scheduled_at: event.target.value })
            }}
          />
        </label>
        <DraftText
          label={t('workPlan.note')}
          value={plan.note ?? ''}
          rows={2}
          placeholder={t('workPlan.notePlaceholder')}
          onCommit={(note) => save.mutate({ id: plan.id, note: note.trim() === '' ? null : note })}
        />
        <div className="flex flex-wrap gap-2">
          {ownDoor ? (
            <Button
              variant="primary"
              disabled={fulfill.isPending}
              onClick={() => fulfill.mutate({ id: plan.id, kind: null })}
            >
              {t('workPlan.bookOwn')}
            </Button>
          ) : (
            makeable.map((kind) => (
              <Button
                key={kind.key}
                variant="primary"
                disabled={fulfill.isPending}
                onClick={() => fulfill.mutate({ id: plan.id, kind: kind.key })}
              >
                {t('workPlan.make', { kind: sayLabel(kind.label) })}
              </Button>
            ))
          )}
        </div>
        <div className="flex flex-wrap gap-2 border-t border-line pt-3">
          <Button onClick={() => onOpenWork(plan.work_id)}>{t('workPlan.openWork')}</Button>
          <Button
            variant="danger"
            disabled={remove.isPending}
            onClick={() => remove.mutate(plan.id)}
          >
            {t('workPlan.remove')}
          </Button>
        </div>
      </div>
    </Dialog>
  )
}
