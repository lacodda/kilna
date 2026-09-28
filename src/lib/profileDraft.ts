import type { ProfileConfig } from '@/lib/api/types'

/*
 * The profile being edited in Settings, kept until it is saved or thrown away.
 *
 * Until v0.79 the editor held its draft in component state, so leaving the
 * Profile section - for the catalogue, for another section, by a link in a
 * toast - dropped every edit without a word; an action's method is a page of
 * writing, and it went with them. Warning before leaving was the other way
 * out, and a worse one: the window can be left by the rail, the palette, a
 * card link, a shortcut or closing it, and a question at each of those doors
 * is a question asked of someone who has already decided to go. So the draft
 * is kept instead - in `localStorage`, beside the version drafts
 * (`lib/drafts`), for their reason: it survives a restart, costs no schema,
 * and is gone the moment it is saved or discarded.
 *
 * A kept draft has one hazard a component's state never had: the profile can
 * change under it. The catalogue writes its columns into the same document,
 * and a new build carries new actions into it on start (`carry_forward`).
 * A draft that was the whole document as it stood would put all of that back
 * the way it was on save. So it is kept with the document it started from,
 * and rebased onto what is stored each time it is read: what the person
 * changed is theirs, what they did not touch follows the stored copy.
 */

const PREFIX = 'kilna.profileDraft'

/** What is kept: the draft, and the stored document it was made against. */
export interface ProfileDraft {
  base: ProfileConfig
  config: ProfileConfig
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

/** Absent, `undefined` and `null` are one answer in a profile: the backend
 *  reads a missing field and a null one as the same `None`, and the editor
 *  writes `null` or `undefined` where a box was emptied. */
function isNothing(value: unknown): value is null | undefined {
  return value === null || value === undefined
}

/** Whether two parts of a profile say the same thing - by value, key order
 *  aside, with an emptied field the same as one never written. */
export function sameValue(a: unknown, b: unknown): boolean {
  // The stored profile against itself, on every render without a draft.
  if (a === b) return true
  if (isNothing(a) || isNothing(b)) return isNothing(a) && isNothing(b)
  if (Array.isArray(a) || Array.isArray(b)) {
    return (
      Array.isArray(a) &&
      Array.isArray(b) &&
      a.length === b.length &&
      a.every((item, index) => sameValue(item, b[index]))
    )
  }
  if (isRecord(a) && isRecord(b)) {
    for (const key of new Set([...Object.keys(a), ...Object.keys(b)])) {
      if (!sameValue(a[key], b[key])) return false
    }
    return true
  }
  return Object.is(a, b)
}

interface Keyed {
  key: string
}

/** A list whose entries name themselves: kinds, axes, tiers, actions. Its
 *  entries are matched by key rather than by place, since both sides may have
 *  added or removed some. */
function isKeyedList(value: unknown): value is Keyed[] {
  return (
    Array.isArray(value) && value.every((item) => isRecord(item) && typeof item.key === 'string')
  )
}

/**
 * `ours` - an edit of `base` - carried onto `theirs`, what `base` has become
 * since.
 *
 * A three-way merge, part by part. A part only one side changed takes that
 * side's value. A part both changed is merged inside when it is a document or
 * a keyed list, and otherwise takes ours: the person's edit is the one thing
 * here somebody chose on purpose, and the screen they are looking at shows it.
 * A list of plain values - the columns of the catalogue, the kinds an action
 * is for - is one value, as its order is part of what it says.
 */
export function rebase(base: unknown, ours: unknown, theirs: unknown): unknown {
  if (sameValue(ours, base)) return theirs
  if (sameValue(theirs, base) || sameValue(ours, theirs)) return ours

  if (isRecord(ours) && isRecord(theirs)) {
    const was = isRecord(base) ? base : {}
    const merged: Record<string, unknown> = {}
    for (const key of new Set([...Object.keys(ours), ...Object.keys(theirs)])) {
      const value = rebase(was[key], ours[key], theirs[key])
      if (value !== undefined) merged[key] = value
    }
    return merged
  }

  if (isKeyedList(ours) && isKeyedList(theirs)) {
    const byKey = (list: Keyed[]) => new Map(list.map((item) => [item.key, item]))
    const was = byKey(isKeyedList(base) ? base : [])
    const mine = byKey(ours)
    const now = byKey(theirs)
    const merged: unknown[] = []
    // Ours in our order, each carried onto its stored self; an entry the
    // stored side removed goes with it unless we changed it.
    for (const item of ours) {
      const value = rebase(was.get(item.key), item, now.get(item.key))
      if (!isNothing(value)) merged.push(value)
    }
    // Then what only the stored side has: added there, it joins at the end;
    // removed here, it stays removed unless it was changed there.
    for (const item of theirs) {
      if (mine.has(item.key)) continue
      const value = rebase(was.get(item.key), undefined, item)
      if (!isNothing(value)) merged.push(value)
    }
    return merged
  }

  return ours
}

/** The kept draft as it reads against what is stored now. */
export function rebaseDraft(draft: ProfileDraft, stored: ProfileConfig): ProfileConfig {
  return rebase(draft.base, draft.config, stored) as ProfileConfig
}

/** The kept text, read back. Anything that is not a draft - a slot written by
 *  an older build, a truncated write - reads as none rather than as a crash. */
export function parseProfileDraft(text: string | null): ProfileDraft | null {
  if (text === null) return null
  try {
    const value: unknown = JSON.parse(text)
    if (isRecord(value) && isRecord(value.base) && isRecord(value.config)) {
      return value as unknown as ProfileDraft
    }
  } catch {
    // Falls through to "no draft".
  }
  return null
}

// Every screen that shows the draft reads the same slot, so a write is told to
// all of them: the editor, the bar under it, the mark in the section list.
const listeners = new Set<() => void>()

// What was last written, for when storage refuses: a full disk must not turn
// every keystroke in the editor into one that is undone as it is typed.
const memory = new Map<string, string | null>()
let storageRefused = false

function slot(profileId: string): string {
  return `${PREFIX}.${profileId}`
}

export function subscribeProfileDraft(listener: () => void): () => void {
  listeners.add(listener)
  return () => listeners.delete(listener)
}

/**
 * The kept draft as its stored text, or `null` when there is none.
 *
 * Text rather than the parsed draft, because this is what a
 * `useSyncExternalStore` snapshot has to be: the same value for as long as
 * nothing was written, and a string compares by what it says.
 */
export function readProfileDraft(profileId: string): string | null {
  if (!storageRefused) {
    try {
      return localStorage.getItem(slot(profileId))
    } catch {
      storageRefused = true
    }
  }
  return memory.get(slot(profileId)) ?? null
}

/** Keep `draft` for the profile, or drop the kept one with `null`. Text is
 *  accepted as it was read, to put back a draft that was just discarded. */
export function keepProfileDraft(profileId: string, draft: ProfileDraft | string | null): void {
  const text = draft === null || typeof draft === 'string' ? draft : JSON.stringify(draft)
  memory.set(slot(profileId), text)
  if (!storageRefused) {
    try {
      if (text === null) localStorage.removeItem(slot(profileId))
      else localStorage.setItem(slot(profileId), text)
    } catch {
      storageRefused = true
    }
  }
  for (const listener of listeners) listener()
}
