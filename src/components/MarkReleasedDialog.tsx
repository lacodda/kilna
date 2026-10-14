import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { ScheduledRelease } from '@/lib/api/types'
import { isWebLink } from '@/lib/link'
import { readLink } from '@/lib/releaseLink'
import { say as sayLabel, useProfile, vocabularyOf } from '@/lib/useProfile'
import { Select } from '@/components/AppSelect'
import { Button } from '@/components/ui/button'
import { DatePicker } from '@/components/DatePicker'
import { Dialog } from '@/components/AppDialog'
import { Field, FieldGroup } from '@/components/ui/field'
import { Input } from '@/components/ui/input'

const FORM = 'mark-released'

interface Props {
  release: ScheduledRelease | null
  /** Today, so the default day comes from one clock rather than each dialog's. */
  today: string
  onOpenChange: (open: boolean) => void
  /** `kind` is the door it went out through, when that is not the one
   *  planned; null when it is. */
  onConfirm: (id: string, url: string | null, at: string | null, kind: string | null) => void
}

/** What the dialog holds, and which of it a person set by hand: what the
 *  link recognises fills only what nobody has touched. */
interface Draft {
  at: string
  url: string
  kind: string
  /** The day was picked by hand. */
  atPicked: boolean
  /** The door was picked by hand. */
  kindPicked: boolean
  /** What was last filled from the link, to say so under the box. */
  fromLink: { at: boolean; kind: boolean }
}

/**
 * Recording that something actually went out: on which day, through which
 * door, at which address.
 *
 * Three facts, each optional in its own way. The link is optional because
 * plenty of releases have no address worth keeping, and leaving it empty
 * keeps whatever link is already recorded rather than clearing it. The day
 * defaults to the planned one, else today, because the mark is often made
 * after the fact. The door defaults to the planned one.
 *
 * A link pasted here is read (v0.97): the door whose addresses it looks like
 * is picked, and a day its path carries is taken - each only while nobody
 * has picked that box by hand, and each said under its box, so a guess is
 * seen before it is saved.
 */
export function MarkReleasedDialog({ release, today, onOpenChange, onConfirm }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const doors = vocabularyOf(profile.config, release?.work_kind).release_kinds

  const [draft, setDraft] = useState(() => toDraft(release, today, doors))
  const [syncedTo, setSyncedTo] = useState(release?.id ?? null)

  if (syncedTo !== (release?.id ?? null)) {
    setSyncedTo(release?.id ?? null)
    setDraft(toDraft(release, today, doors))
  }

  const linkLooksWrong = draft.url !== '' && !isWebLink(draft.url)

  const typeLink = (url: string) =>
    setDraft((current) => {
      const reading = readLink(url, doors)
      const kind = !current.kindPicked && reading.kind !== null ? reading.kind : current.kind
      const at = !current.atPicked && reading.day !== null ? reading.day : current.at
      return {
        ...current,
        url,
        kind,
        at,
        fromLink: {
          kind: !current.kindPicked && reading.kind !== null && reading.kind !== release?.kind,
          at: !current.atPicked && reading.day !== null,
        },
      }
    })

  // The door is a choice only where the work has more than one.
  const offered = doors.map((door) => ({ value: door.key, label: sayLabel(door.label) }))
  if (draft.kind !== '' && !offered.some((option) => option.value === draft.kind)) {
    offered.push({ value: draft.kind, label: draft.kind })
  }

  return (
    <Dialog
      open={release !== null}
      onOpenChange={onOpenChange}
      title={t('calendar.markReleased')}
      description={release?.work_title}
      // The dialog's own row holds Cancel; the form's submit sits beside it
      // rather than in a second row of its own under a second Cancel.
      footer={
        <Button type="submit" form={FORM} variant="primary">
          {t('calendar.markReleased')}
        </Button>
      }
    >
      <form
        id={FORM}
        className="flex flex-col gap-3"
        onSubmit={(event) => {
          event.preventDefault()
          if (release === null) return
          onConfirm(
            release.id,
            draft.url.trim() === '' ? null : draft.url.trim(),
            draft.at === '' ? null : draft.at,
            draft.kind === '' || draft.kind === release.kind ? null : draft.kind,
          )
          onOpenChange(false)
        }}
      >
        <Field
          label={t('calendar.urlPrompt')}
          help={linkLooksWrong ? t('releases.linkLooksWrong') : undefined}
        >
          <Input
            autoFocus
            value={draft.url}
            onChange={(event) => typeLink(event.target.value)}
            placeholder="https://"
          />
        </Field>

        {offered.length > 1 && (
          <FieldGroup
            label={t('releases.wentThrough')}
            help={draft.fromLink.kind ? t('releases.recognisedFromLink') : undefined}
          >
            <Select
              aria-label={t('releases.wentThrough')}
              className="w-full"
              value={draft.kind}
              onChange={(kind) =>
                setDraft((current) => ({
                  ...current,
                  kind,
                  kindPicked: true,
                  fromLink: { ...current.fromLink, kind: false },
                }))
              }
              options={offered}
            />
          </FieldGroup>
        )}

        <FieldGroup
          label={t('releases.wentOutOn')}
          help={draft.fromLink.at ? t('releases.recognisedFromLink') : t('releases.wentOutOnHint')}
        >
          <DatePicker
            className="w-full"
            value={draft.at}
            onChange={(at) =>
              setDraft((current) => ({
                ...current,
                at,
                atPicked: true,
                fromLink: { ...current.fromLink, at: false },
              }))
            }
            placeholder={t('releases.wentOutOn')}
            aria-label={t('releases.wentOutOn')}
          />
        </FieldGroup>
      </form>
    </Dialog>
  )
}

/**
 * Where the dialog starts: the planned day and door - the likeliest day it
 * went out, failing that today - and the link already recorded, read the way
 * a pasted one is: a release whose link was written before its mark opens on
 * the door and the day the link says.
 */
function toDraft(
  release: ScheduledRelease | null,
  today: string,
  doors: Parameters<typeof readLink>[1],
): Draft {
  const url = release?.url ?? ''
  const planned = release?.scheduled_at ?? today
  const reading = url === '' ? { kind: null, day: null } : readLink(url, doors)
  return {
    at: reading.day ?? planned,
    url,
    kind: reading.kind ?? release?.kind ?? '',
    atPicked: false,
    kindPicked: false,
    fromLink: {
      at: reading.day !== null && reading.day !== planned,
      kind: reading.kind !== null && reading.kind !== release?.kind,
    },
  }
}
