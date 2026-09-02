import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { ScheduledRelease } from '@/lib/api'
import { isWebLink } from '@/lib/link'
import { Button } from '@/components/ui/button'
import { DatePicker } from '@/components/ui/DatePicker'
import { Dialog } from '@/components/ui/AppDialog'
import { Field } from '@/components/ui/Field'
import { Input } from '@/components/ui/input'

interface Props {
  release: ScheduledRelease | null
  /** Today, so the default day comes from one clock rather than each dialog's. */
  today: string
  onOpenChange: (open: boolean) => void
  onConfirm: (id: string, url: string | null, at: string | null) => void
}

/**
 * Recording that something actually went out.
 *
 * Two facts, and both are optional in different ways. The link is optional
 * because plenty of releases have no address worth keeping, and leaving it
 * empty keeps whatever link is already recorded rather than clearing it. The
 * day defaults to today but can be changed, because the mark is often made
 * after the fact - the calendar's own prompt could only ever say "now", which
 * made every late mark quietly wrong.
 */
export function MarkReleasedDialog({ release, today, onOpenChange, onConfirm }: Props) {
  const { t } = useTranslation()

  const [draft, setDraft] = useState(() => toDraft(release, today))
  const [syncedTo, setSyncedTo] = useState(release?.id ?? null)

  if (syncedTo !== (release?.id ?? null)) {
    setSyncedTo(release?.id ?? null)
    setDraft(toDraft(release, today))
  }

  const linkLooksWrong = draft.url !== '' && !isWebLink(draft.url)

  return (
    <Dialog
      open={release !== null}
      onOpenChange={onOpenChange}
      title={t('calendar.markReleased')}
      description={release?.work_title}
    >
      <form
        className="flex flex-col gap-3"
        onSubmit={(event) => {
          event.preventDefault()
          if (release === null) return
          onConfirm(
            release.id,
            draft.url === '' ? null : draft.url,
            draft.at === '' ? null : draft.at,
          )
          onOpenChange(false)
        }}
      >
        <Field label={t('releases.wentOutOn')} hint={t('releases.wentOutOnHint')}>
          <DatePicker
            className="w-full"
            value={draft.at}
            onChange={(at) => setDraft((current) => ({ ...current, at }))}
            placeholder={t('releases.wentOutOn')}
            aria-label={t('releases.wentOutOn')}
          />
        </Field>

        <Field
          label={t('calendar.urlPrompt')}
          hint={linkLooksWrong ? t('releases.linkLooksWrong') : undefined}
        >
          <Input
            autoFocus
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
            {t('calendar.markReleased')}
          </Button>
        </div>
      </form>
    </Dialog>
  )
}

function toDraft(release: ScheduledRelease | null, today: string) {
  return {
    // The day it was planned for is the likeliest day it went out; failing
    // that, today.
    at: release?.scheduled_at ?? today,
    url: release?.url ?? '',
  }
}
