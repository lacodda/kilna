import type { Dispatch, SetStateAction } from 'react'
import { useTranslation } from 'react-i18next'
import type { ScheduledRelease } from '@/lib/api/types'
import { today } from '@/lib/month'
import { say } from '@/lib/toast'
import { useProfile, vocabularyOf } from '@/lib/useProfile'
import { ConfirmAction } from '@/components/ConfirmAction'
import { MarkReleasedDialog } from '@/components/MarkReleasedDialog'
import { PickWorkDialog } from '@/components/PickWorkDialog'
import { ReleaseEditor } from '@/features/calendar/ReleaseEditor'
import type { CalendarActions } from '@/features/calendar/useCalendarActions'

/**
 * Which of the calendar's dialogs is open, and on what. One at a time, which
 * is how they are used: the editor hands over to the release dialog, and
 * nothing else opens over another.
 */
export type CalendarDialog =
  | { kind: 'edit'; releaseId: string }
  | { kind: 'release'; releaseId: string }
  | { kind: 'fill'; date: string }
  | { kind: 'fields' }
  | null

interface Props {
  dialog: CalendarDialog
  /** The screen's setter: closing reads what is open NOW, not at render. */
  onDialog: Dispatch<SetStateAction<CalendarDialog>>
  /** Everything with a date, for the release a dialog is about. */
  slots: readonly ScheduledRelease[]
  /** The month's planned releases the fields batch writes. */
  planned: string[]
  actions: CalendarActions
  onOpenWork: (workId: string) => void
}

/**
 * The calendar's dialogs, wired to the writes they end in.
 *
 * Each closes only itself: the editor's "Mark released" opens the release
 * dialog and then closes the editor, in one batch, and a close that took
 * whatever was open would take the dialog it had just handed over to. So a
 * close asks the state as it stands when it runs, not as it stood at render.
 */
export function CalendarDialogs({ dialog, onDialog, slots, planned, actions, onOpenWork }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()

  /** Close `kind` if it is still the one open. */
  const close = (kind: NonNullable<CalendarDialog>['kind']) => (open: boolean) => {
    if (!open) onDialog((current) => (current?.kind === kind ? null : current))
  }

  // Found afresh on every render rather than held in state: pinning from
  // inside the dialog otherwise left the tick unmoved until it was closed and
  // opened again. An invalidated query keeps serving what it has while it
  // refetches, so the row does not vanish out from under the dialog - checked
  // by pinning with the dialog open.
  const find = (id: string) => slots.find((entry) => entry.id === id) ?? null

  return (
    <>
      {/* The day's plus: which work goes out here. Booking it is one call -
          a release of the work's own first door, dated to the day that was
          pressed. A work whose kind has no door cannot go out at all, and
          says so rather than booking nothing. */}
      <PickWorkDialog
        open={dialog?.kind === 'fill'}
        onOpenChange={close('fill')}
        title={t('calendar.fillDay', { date: dialog?.kind === 'fill' ? dialog.date : '' })}
        onPick={(work) => {
          if (dialog?.kind !== 'fill') return
          const door = vocabularyOf(profile.config, work.kind).release_kinds[0]
          if (door === undefined) {
            say.warn(t('calendar.noDoor', { title: work.title }))
            return
          }
          actions.fillDay.mutate({
            work_id: work.work_id,
            kind: door.key,
            scheduled_at: dialog.date,
          })
        }}
      />

      <ReleaseEditor
        release={dialog?.kind === 'edit' ? find(dialog.releaseId) : null}
        onOpenChange={close('edit')}
        onSaved={actions.settle}
        onOpenWork={onOpenWork}
        onMarkReleased={(id) => onDialog({ kind: 'release', releaseId: id })}
        onUnschedule={(id) => actions.unschedule.mutate(id)}
      />

      {/* The same dialog the Releases tab marks with. The calendar had its
          own prompt that asked only for the link, so the day a release went
          out was always the moment of the click - and a mark made the day
          after was quietly wrong (decision 02.09: the person names the day). */}
      <MarkReleasedDialog
        release={dialog?.kind === 'release' ? find(dialog.releaseId) : null}
        today={today()}
        onOpenChange={close('release')}
        onConfirm={(id, url, at) => actions.release.mutate({ id, url, at })}
      />

      {/* Always asked, unlike the single release's button: this replaces the
          wording of a whole month at once, and that is a great deal to walk
          back one release at a time. */}
      <ConfirmAction
        open={dialog?.kind === 'fields'}
        onOpenChange={close('fields')}
        title={t('calendar.fields.title')}
        description={t('calendar.fields.body', { count: planned.length })}
        actionLabel={t('calendar.fields.confirm')}
        onConfirm={() => {
          onDialog(null)
          actions.fillFields.mutate(planned)
        }}
      />
    </>
  )
}
