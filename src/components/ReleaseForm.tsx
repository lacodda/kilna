import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { ScheduledRelease } from '@/lib/api/types'
import { isWebLink } from '@/lib/link'
import { draftOf, rebase, sameDraft, type ReleaseDraft } from '@/lib/releaseForm'
import { say, useProfile, vocabularyOf } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Select } from '@/components/AppSelect'
import { DatePicker } from '@/components/DatePicker'
import { Checkbox } from '@/components/ui/checkbox'
import { Field, FieldGroup } from '@/components/ui/field'
import { Input } from '@/components/ui/input'

interface Props {
  release: ScheduledRelease
  value: ReleaseDraft
  /** Every edit, keystroke by keystroke. */
  onChange: (draft: ReleaseDraft) => void
  /**
   * A field is finished with: a kind or a day picked, the date kept or let
   * go, the link left. Where the form saves as it goes - a release's row -
   * this is the save; a dialog waits for its own Save and leaves it out.
   */
  onSettle?: (draft: ReleaseDraft) => void
  /** `row`: side by side, in a release's unrolled row. `stack`: one under
   * another, in a dialog's column. */
  layout?: 'row' | 'stack'
  className?: string
}

/**
 * A release's own facts - its kind, its date and whether the date is kept,
 * its link - the one form they are edited through.
 *
 * Two places drew this form until v0.80, the Releases tab's dialog and the
 * calendar's, and they had drifted: the calendar's once offered the release
 * kinds of every kind of work, so a song could be set to go out as a video
 * premiere; only the tab's warned about a link that would not open. Now both
 * places draw this, and what a draft means - which boxes count as a change,
 * what an empty one says - is `lib/releaseForm`.
 *
 * The kinds offered are the work's own. A release is always of one work, and
 * a kind its work cannot go out through is not a choice but a mistake.
 *
 * A date typed here is a correction, not a bid for a slot: nothing is
 * contested and nothing pushes back. Taking a day from another release is
 * what dragging on the calendar does, through the contest.
 */
export function ReleaseForm({
  release,
  value,
  onChange,
  onSettle,
  layout = 'stack',
  className,
}: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const kinds = vocabularyOf(profile.config, release.work_kind).release_kinds

  // A kind the profile no longer names is still what the release is: offered
  // under its raw key, so the box shows it rather than a blank.
  const options = kinds.map((kind) => ({ value: kind.key, label: say(kind.label) }))
  if (value.kind !== '' && !options.some((option) => option.value === value.kind)) {
    options.push({ value: value.kind, label: value.kind })
  }

  const change = (patch: Partial<ReleaseDraft>) => onChange({ ...value, ...patch })
  const settle = (patch: Partial<ReleaseDraft>) => {
    const next = { ...value, ...patch }
    onChange(next)
    onSettle?.(next)
  }

  // A warning, not a refusal: what someone typed is kept, and the note says
  // why the link will not open. Refusing to save would lose the text they have.
  const linkLooksWrong = value.url.trim() !== '' && !isWebLink(value.url)
  const row = layout === 'row'
  const usualTime = profile.config.rhythm?.default_time

  return (
    <div
      className={cn(
        row ? 'flex flex-wrap items-start gap-x-2.5 gap-y-2' : 'flex flex-col gap-3',
        className,
      )}
    >
      <FieldGroup label={t('releases.kind')} className={cn(row && 'w-44')}>
        <Select
          aria-label={t('releases.kind')}
          className="w-full"
          value={value.kind}
          onChange={(kind) => settle({ kind })}
          options={options}
        />
      </FieldGroup>

      <FieldGroup
        label={t('calendar.slotDate')}
        // The profile's usual shipping time, as a reminder beside the date.
        // Slots stay whole days - the contest is per day, and a time would
        // split it - so the time lives in the profile, not on the release.
        help={
          !row && typeof usualTime === 'string'
            ? t('calendar.usualTime', { time: usualTime })
            : undefined
        }
        className={cn(row && 'w-40')}
      >
        <DatePicker
          className="w-full"
          value={value.date}
          onChange={(date) => settle({ date })}
          placeholder={t('releases.unscheduled')}
          aria-label={t('calendar.slotDate')}
        />
        {/* Keeping the date belongs beside the date rather than among the
            actions: it is a property of this day, not something done to the
            release. There is nothing to keep until there is a day, and
            nothing to keep it from once the release has gone out. */}
        {value.date !== '' && release.status !== 'released' && (
          <Checkbox checked={value.pinned} onCheckedChange={(pinned) => settle({ pinned })}>
            <span>
              {t('calendar.pinSlot')}
              {!row && (
                <span className="block text-xs text-faint">{t('calendar.pinSlotHint')}</span>
              )}
            </span>
          </Checkbox>
        )}
      </FieldGroup>

      <Field
        label={t('calendar.urlPrompt')}
        help={linkLooksWrong ? t('releases.linkLooksWrong') : undefined}
        className={cn(row && 'min-w-56 flex-1')}
      >
        <Input
          value={value.url}
          onChange={(event) => change({ url: event.target.value })}
          onBlur={() => onSettle?.(value)}
          onKeyDown={(event) => {
            // Enter is the end of typing a link in a form that saves as it
            // goes; in a dialog it is the dialog's Save.
            if (event.key === 'Enter' && onSettle !== undefined) {
              event.preventDefault()
              onSettle(value)
            }
          }}
          placeholder="https://"
        />
      </Field>
    </div>
  )
}

/**
 * The draft a form edits, drawn from the release and following it.
 *
 * It follows what is stored, not only which release is open: a date moved
 * on the calendar, an undo, the save of this very form coming back - each is
 * the release's new truth, and a form still showing the old one would save it
 * back over the new at the next blur. Only the fields the store moved follow
 * it (`rebase`); one being typed in stays as typed.
 */
export function useReleaseDraft(release: ScheduledRelease | null) {
  const id = release?.id ?? null
  const stored = release === null ? null : draftOf(release)
  const [state, setState] = useState({ id, stored, draft: stored })

  let current = state
  if (state.id !== id) {
    // Another release: start from what it holds, not from the last one's edit.
    current = { id, stored, draft: stored }
  } else if (
    state.stored !== null &&
    state.draft !== null &&
    stored !== null &&
    !sameDraft(state.stored, stored)
  ) {
    current = { id, stored, draft: rebase(state.draft, state.stored, stored) }
  }
  // Settled during render, as React has a component adjust its state to new
  // props: the first paint after the move already shows it.
  if (current !== state) setState(current)

  const setDraft = (draft: ReleaseDraft) => setState((now) => ({ ...now, draft }))
  return [current.draft, setDraft] as const
}
