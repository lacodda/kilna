import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { ReleasePatch, ScheduledRelease } from '@/lib/api'
import { isWebLink } from '@/lib/link'
import { useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { DatePicker } from '@/components/ui/DatePicker'
import { Dialog } from '@/components/ui/AppDialog'
import { Field } from '@/components/ui/Field'
import { Input } from '@/components/ui/input'
import { Select } from '@/components/ui/AppSelect'

interface Props {
  release: ScheduledRelease | null
  onOpenChange: (open: boolean) => void
  onSave: (id: string, patch: ReleasePatch) => void
}

/**
 * Editing a release from the tab of the work it belongs to.
 *
 * The same three fields the calendar's dialog offers - kind, date, link - and
 * for the same reason: a date typed into a form is a correction rather than a
 * bid for a slot, so nothing is contested and nothing pushes back.
 *
 * What it deliberately does not carry are the calendar's own actions. Marking
 * released, unscheduling and deleting live in the row's menu, where they read
 * as things done to a release rather than as fields of it.
 */
export function ReleaseRowEditor({ release, onOpenChange, onSave }: Props) {
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

  // A warning, not a refusal: what someone typed is kept, and the note says why
  // the link will not open. Refusing to save would lose the text they have.
  const linkLooksWrong = draft.url !== '' && !isWebLink(draft.url)

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
          if (release === null) return
          onSave(release.id, {
            kind: draft.kind,
            // An empty date clears the slot, which is the same as returning it
            // to the queue - a release without a date is an intention.
            scheduled_at: draft.scheduled_at === '' ? null : draft.scheduled_at,
            url: draft.url === '' ? null : draft.url,
          })
          onOpenChange(false)
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

        <Field label={t('calendar.urlPrompt')} hint={linkLooksWrong ? t('releases.linkLooksWrong') : undefined}>
          <Input
            value={draft.url}
            onChange={(event) => setDraft((current) => ({ ...current, url: event.target.value }))}
            placeholder="https://"
          />
        </Field>

        <div className="mt-1 flex justify-end gap-2">
          <Button type="button" onClick={() => onOpenChange(false)}>
            {t('dialog.cancel')}
          </Button>
          <Button type="submit" variant="primary">
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
