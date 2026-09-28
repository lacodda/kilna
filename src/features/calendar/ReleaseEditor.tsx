import { useTranslation } from 'react-i18next'
import { useQueryClient } from '@tanstack/react-query'
import { saveRelease } from '@/lib/api/releases'
import type { ScheduledRelease } from '@/lib/api/types'
import { announceEdited } from '@/lib/edited'
import { missing } from '@/lib/readiness'
import { openExternal } from '@/lib/link'
import { keys } from '@/lib/query/keys'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { changesOf, type ReleaseChanges } from '@/lib/releaseForm'
import { labelOf, useProfile, vocabularyOf } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Dialog } from '@/components/AppDialog'
import { ReleaseForm, useReleaseDraft } from '@/components/ReleaseForm'

interface Props {
  release: ScheduledRelease | null
  onOpenChange: (open: boolean) => void
  onSaved: () => void
  /** Opens the work behind this release. */
  onOpenWork: (workId: string) => void
  onMarkReleased: (releaseId: string) => void
  onUnschedule: (releaseId: string) => void
}

/** What an edit made here disturbs: both sides of the calendar, and the
    work's own list of releases. */
const REFRESHED = [keys.calendar, keys.releaseQueue, keys.releases] as const

/**
 * Editing a booking you already hold: its kind, its date and whether the date
 * is kept, the link.
 *
 * The form is the one the Releases tab unrolls in a release's row
 * (`components/ReleaseForm`); what this dialog adds is the Save the form is
 * written with, and the things done to a release rather than written into
 * it. Everything the form holds is saved together, the pin included: until
 * v0.80 the pin was written the moment it was ticked while the date beside
 * it waited for Save, and Cancel took back one and not the other.
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
}: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const client = useQueryClient()
  const versionRoles = vocabularyOf(profile.config, release?.work_kind).version_roles

  const [draft, setDraft] = useReleaseDraft(release)
  const changes = release === null || draft === null ? null : changesOf(release, draft)

  const save = useAppMutation({
    mutationFn: ({ id, changes }: { id: string; changes: ReleaseChanges }) =>
      saveRelease(id, changes),
    failure: 'toast.releaseSaveFailed',
    refresh: REFRESHED,
    onSuccess: () => {
      // The parent clears what its own state says about the month - a plan
      // previewed over it - and checks the week ahead for what is unready.
      onSaved()
      onOpenChange(false)
      announceEdited({ client, message: t('toast.releaseSaved'), refresh: [] })
    },
  })

  /**
   * An action done to the release, after what was typed is kept. Each one
   * closes the dialog, since each leaves it describing something no longer
   * true - and closing used to throw away a date moved a moment before the
   * button was pressed.
   */
  const thenDo = (act: (entry: ScheduledRelease) => void) => {
    if (release === null) return
    if (changes === null) {
      act(release)
      onOpenChange(false)
      return
    }
    save.mutate({ id: release.id, changes }, { onSuccess: () => act(release) })
  }

  return (
    <Dialog
      open={release !== null}
      onOpenChange={onOpenChange}
      title={release?.work_title ?? ''}
      // A picked kind or day is an edit as much as a typed link, and a click
      // beside the dialog must not throw it away.
      dirty={changes !== null}
      // One row of answers: the dialog's Cancel and this. The form drew its
      // own Cancel and Save, and the dialog added a second Cancel under them.
      footer={
        <Button type="submit" form={FORM} variant="primary" disabled={save.isPending}>
          {t('dialog.save')}
        </Button>
      }
    >
      {release !== null && draft !== null && (
        <form
          id={FORM}
          className="flex flex-col gap-3"
          onSubmit={(event) => {
            event.preventDefault()
            // Nothing changed is nothing to write: the dialog just closes,
            // with no journal line and no undo for an edit that was not made.
            if (changes === null) onOpenChange(false)
            else save.mutate({ id: release.id, changes })
          }}
        >
          <ReleaseForm release={release} value={draft} onChange={setDraft} />

          {/* The chip's glyphs, in words: what this release still needs. Only
              gaps are worth a line — a ready release says nothing here. */}
          {release.status !== 'released' && missing(release.readiness).length > 0 && (
            <p className="text-sm text-warn">
              {t('calendar.notReadyHint', {
                list: missing(release.readiness)
                  .map((gap) =>
                    gap === 'score' ? t('calendar.missingScore') : labelOf(versionRoles, gap),
                  )
                  .join(', '),
              })}
            </p>
          )}

          {/* What can be done to the release itself, rather than written into
              it. */}
          {release.status !== 'released' && (
            <div className="flex flex-wrap gap-2 border-t border-line pt-3">
              <Button
                size="sm"
                disabled={save.isPending}
                onClick={() => thenDo((entry) => onMarkReleased(entry.id))}
              >
                {t('calendar.markReleased')}
              </Button>
              <Button
                size="sm"
                disabled={save.isPending}
                onClick={() => thenDo((entry) => onUnschedule(entry.id))}
              >
                {t('calendar.unschedule')}
              </Button>
              <Button
                size="sm"
                className="ml-auto"
                disabled={save.isPending}
                onClick={() => thenDo((entry) => onOpenWork(entry.work_id))}
              >
                {t('calendar.openWork')}
              </Button>
            </div>
          )}

          {release.status === 'released' && release.url !== null && (
            <p className="border-t border-line pt-3 text-sm text-good">
              {t('calendar.released')}{' '}
              <Button
                variant="link"
                onClick={() => void openExternal(release.url ?? '')}
                title={release.url}
              >
                {t('calendar.link')}
              </Button>
            </p>
          )}
        </form>
      )}
    </Dialog>
  )
}

const FORM = 'release-editor'
