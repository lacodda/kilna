import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation } from '@tanstack/react-query'
import { updateRelease, type ScheduledRelease } from '@/lib/api'
import { say } from '@/lib/toast'
import { useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/Button'
import { DatePicker } from '@/components/ui/DatePicker'
import { Dialog } from '@/components/ui/Dialog'
import { Field, Input } from '@/components/ui/Input'
import { Select } from '@/components/ui/Select'

interface Props {
  release: ScheduledRelease | null
  onOpenChange: (open: boolean) => void
  onSaved: () => void
  /** Opens the work behind this release. */
  onOpenWork: (workId: string) => void
  onMarkReleased: (releaseId: string) => void
  onUnschedule: (releaseId: string) => void
  onTogglePin: (releaseId: string, pinned: boolean) => void
}

/**
 * Editing a booking you already hold: its kind, its date, the link.
 *
 * `update_release` has been on the backend since v0.4.0 with nothing calling
 * it — until now the only way to move a release was to return it to the queue
 * and claim a new slot, which loses the plan on the way.
 *
 * Moving a date here does not compete for the slot the way claiming one does.
 * A date typed into a form is a correction, not a bid, and the app pushing back
 * mid-edit would be answering a question nobody asked.
 */
export function ReleaseEditor({
  release,
  onOpenChange,
  onSaved,
  onOpenWork,
  onMarkReleased,
  onUnschedule,
  onTogglePin,
}: Props) {
  const { t } = useTranslation()
  const profile = useProfile()

  // Keyed by the release, so opening a different one starts from its values
  // rather than the last one's.
  const [draft, setDraft] = useState(() => toDraft(release))
  const [syncedTo, setSyncedTo] = useState(release?.id ?? null)

  if (syncedTo !== (release?.id ?? null)) {
    setSyncedTo(release?.id ?? null)
    setDraft(toDraft(release))
  }

  const save = useMutation({
    mutationFn: () => {
      if (release === null) throw new Error('nothing to save')
      return updateRelease(release.id, {
        kind: draft.kind,
        // An empty date clears the slot, which is the same as returning it to
        // the queue — a release without a date is an intention.
        scheduled_at: draft.scheduled_at === '' ? null : draft.scheduled_at,
        url: draft.url === '' ? null : draft.url,
      })
    },
    onSuccess: () => {
      onSaved()
      onOpenChange(false)
      say.ok(t('toast.releaseSaved'))
    },
    onError: (cause) => say.failedTo(t('toast.releaseSaveFailed'), cause),
  })

  return (
    <Dialog
      open={release !== null}
      onOpenChange={onOpenChange}
      title={release?.work_title ?? ''}
    >
      <form
        className="flex flex-col gap-3"
        onSubmit={(event) => {
          event.preventDefault()
          save.mutate()
        }}
      >
        <Field label={t('releases.kind')}>
          <Select
            className="w-full"
            value={draft.kind}
            onChange={(kind) => setDraft((current) => ({ ...current, kind }))}
            options={profile.config.release_kinds.map((k) => ({ value: k.key, label: k.label }))}
          />
        </Field>

        <Field label={t('calendar.slotDate')} hint={t('calendar.clearDateHint')}>
          <DatePicker
            className="w-full"
            value={draft.scheduled_at}
            onChange={(scheduled_at) => setDraft((current) => ({ ...current, scheduled_at }))}
            placeholder={t('calendar.slotDate')}
            aria-label={t('calendar.slotDate')}
          />
        </Field>

        <Field label={t('calendar.urlPrompt')}>
          <Input
            value={draft.url}
            onChange={(event) => setDraft((current) => ({ ...current, url: event.target.value }))}
            placeholder="https://"
          />
        </Field>

        {/* Pinning belongs beside the date rather than among the actions: it
            is a property of this date, not something done to the release. */}
        {release !== null && release.scheduled_at !== null && (
          <label className="flex cursor-pointer items-start gap-2 text-sm">
            <input
              type="checkbox"
              className="mt-0.5 size-3.5 cursor-pointer accent-[var(--accent)]"
              checked={release.slot_pinned_at !== null}
              onChange={(event) => onTogglePin(release.id, event.target.checked)}
            />
            <span>
              {t('calendar.pinSlot')}
              <span className="block text-xs text-faint">{t('calendar.pinSlotHint')}</span>
            </span>
          </label>
        )}

        {/* What can be done to the release itself, rather than to this form.
            They close the dialog because each one leaves it describing
            something that is no longer true. */}
        {release !== null && release.status !== 'released' && (
          <div className="flex flex-wrap gap-2 border-t border-line pt-3">
            <Button
              size="sm"
              onClick={() => {
                onMarkReleased(release.id)
                onOpenChange(false)
              }}
            >
              {t('calendar.markReleased')}
            </Button>
            <Button
              size="sm"
              onClick={() => {
                onUnschedule(release.id)
                onOpenChange(false)
              }}
            >
              {t('calendar.unschedule')}
            </Button>
            <Button
              size="sm"
              className="ml-auto"
              onClick={() => {
                onOpenWork(release.work_id)
                onOpenChange(false)
              }}
            >
              {t('calendar.openWork')}
            </Button>
          </div>
        )}

        {release !== null && release.status === 'released' && release.url !== null && (
          <p className="border-t border-line pt-3 text-sm text-good">
            {t('calendar.released')}{' '}
            <a href={release.url} target="_blank" rel="noreferrer" className="underline">
              {t('calendar.link')}
            </a>
          </p>
        )}

        <div className="mt-1 flex justify-end gap-2">
          <Button type="button" onClick={() => onOpenChange(false)}>
            {t('dialog.cancel')}
          </Button>
          <Button type="submit" variant="primary" disabled={save.isPending}>
            {t('dialog.save')}
          </Button>
        </div>
      </form>
    </Dialog>
  )
}

function toDraft(release: ScheduledRelease | null) {
  return {
    kind: release?.kind ?? '',
    scheduled_at: release?.scheduled_at ?? '',
    url: release?.url ?? '',
  }
}
