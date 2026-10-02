import type { Release, ReleasePatch } from '@/lib/api/types'

/*
 * A release's own facts - its kind, its date, whether the date is kept, its
 * link - as a form edits them, wherever it is edited.
 *
 * Two places edit them: the release's row on its work's Releases tab and the
 * calendar's dialog. Until v0.80 each had a form of its own, and they had
 * already drifted - one offered the kinds of every kind of work, one warned
 * about a link that would not open and the other did not. There is one form
 * now (`components/ReleaseForm`), and this is what it means: what a draft
 * starts from, and what of it is a change worth sending.
 */

export interface ReleaseDraft {
  kind: string
  /** `yyyy-mm-dd`, or empty for a release waiting in the queue. */
  date: string
  /** `HH:MM` the platform is told, or empty when the day is enough (v0.90). */
  time: string
  url: string
  /** Whether the date is kept: the auto-layout puts nothing over it. */
  pinned: boolean
}

/** The draft a release is edited from: what it holds now. */
export function draftOf(release: Release): ReleaseDraft {
  return {
    kind: release.kind,
    date: release.scheduled_at ?? '',
    time: release.scheduled_time ?? '',
    url: release.url ?? '',
    pinned: release.slot_pinned_at !== null,
  }
}

/** What saving a draft asks of the backend: two commands, either may be idle. */
export interface ReleaseChanges {
  /** For `update_release`, holding only what changed; null when nothing did. */
  patch: ReleasePatch | null
  /** For `set_slot_pin`; null when the pin stands. */
  pin: boolean | null
}

/**
 * What of a draft differs from the release it was drawn from, or null when
 * nothing does - so a field left the way it was found sends nothing, writes
 * no journal line and offers no undo for a change that never happened.
 *
 * An empty date is no date, which returns the release to the queue; an empty
 * link is no link. A link is trimmed: the spaces around a pasted address are
 * the paste's, not the address's.
 *
 * The pin follows the date. With no date there is nothing to keep, and the
 * backend drops a pin along with the date it held - so none is asked for.
 */
export function changesOf(release: Release, draft: ReleaseDraft): ReleaseChanges | null {
  const patch: ReleasePatch = {}

  // An empty kind is a profile that offers none, not a choice to clear it.
  if (draft.kind !== '' && draft.kind !== release.kind) patch.kind = draft.kind

  const date = draft.date === '' ? null : draft.date
  if (date !== release.scheduled_at) patch.scheduled_at = date

  // A time is a moment in somebody's day: the first one typed is said in the
  // zone this machine is in, and a zone already there stays.
  const time = draft.time.trim() === '' ? null : draft.time.trim()
  if (time !== release.scheduled_time) {
    patch.scheduled_time = time
    if (time !== null && release.time_zone === null) patch.time_zone = localZone()
  }

  const url = draft.url.trim() === '' ? null : draft.url.trim()
  if (url !== release.url) patch.url = url

  const pinnedNow = release.slot_pinned_at !== null
  const pin = date !== null && draft.pinned !== pinnedNow ? draft.pinned : null

  const touched = Object.keys(patch).length > 0
  if (!touched && pin === null) return null
  return { patch: touched ? patch : null, pin }
}

/** Whether two drafts say the same thing about a release. */
export function sameDraft(a: ReleaseDraft, b: ReleaseDraft): boolean {
  return (
    a.kind === b.kind &&
    a.date === b.date &&
    a.time === b.time &&
    a.url === b.url &&
    a.pinned === b.pinned
  )
}

/**
 * A draft carried over a change to the release underneath it, `from` what
 * the release held `to` what it holds now.
 *
 * Field by field: what the store moved takes the stored value - a date moved
 * on the calendar, an undo, this form's own save coming back - and what the
 * store left alone keeps what is being typed in it. Resetting the whole draft
 * instead lost a link half typed whenever the date picked a moment earlier
 * came back saved.
 */
export function rebase(draft: ReleaseDraft, from: ReleaseDraft, to: ReleaseDraft): ReleaseDraft {
  return {
    kind: to.kind !== from.kind ? to.kind : draft.kind,
    date: to.date !== from.date ? to.date : draft.date,
    time: to.time !== from.time ? to.time : draft.time,
    url: to.url !== from.url ? to.url : draft.url,
    pinned: to.pinned !== from.pinned ? to.pinned : draft.pinned,
  }
}

/** The zone this machine is in, as an IANA name: the zone a time typed here
 *  is meant in. UTC when the runtime cannot say. */
export function localZone(): string {
  try {
    return Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC'
  } catch {
    return 'UTC'
  }
}
