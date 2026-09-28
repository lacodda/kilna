import { useCallback, useEffect, useMemo, useSyncExternalStore } from 'react'
import type { ProfileConfig } from '@/lib/api/types'
import {
  keepProfileDraft,
  parseProfileDraft,
  readProfileDraft,
  rebaseDraft,
  sameValue,
  subscribeProfileDraft,
} from '@/lib/profileDraft'
import { useProfile } from '@/lib/useProfile'

export interface ProfileDraftState {
  /** The profile as the editor shows it: the kept draft, or what is stored. */
  config: ProfileConfig
  /** Whether that differs from what is stored - what the bar and the mark in
   *  the section list say. */
  dirty: boolean
  /** Change the draft. Given the draft as it stands at the moment of the call,
   *  so two changes in one event both land. */
  edit: (change: (current: ProfileConfig) => ProfileConfig) => void
  /** Drop the draft, answering with its kept text for `restore`. */
  discard: () => string | null
  /** Put back a draft that `discard` answered with. */
  restore: (text: string) => void
}

/**
 * The active profile's draft (`lib/profileDraft`), for every part of Settings
 * that shows it.
 *
 * The editor, the bar that saves it and the mark on the Profile row each call
 * this and read the same kept slot, so none of them owns the draft and none of
 * them loses it by being taken off the screen.
 */
export function useProfileDraft(): ProfileDraftState {
  const profile = useProfile()
  const stored = profile.config
  const id = profile.id

  const text = useSyncExternalStore(subscribeProfileDraft, () => readProfileDraft(id))

  const config = useMemo(() => {
    const kept = parseProfileDraft(text)
    return kept === null ? stored : rebaseDraft(kept, stored)
  }, [text, stored])
  const dirty = useMemo(() => !sameValue(config, stored), [config, stored])

  // A draft that has come to say what is stored is no draft: it was saved, or
  // someone else stored the same. The slot is emptied so a later change to
  // the profile is not merged against a draft nobody is making.
  useEffect(() => {
    if (text !== null && !dirty) keepProfileDraft(id, null)
  }, [text, dirty, id])

  const edit = useCallback(
    (change: (current: ProfileConfig) => ProfileConfig) => {
      // Read again rather than taken from the render: the render may be one
      // keystroke behind the slot.
      const kept = parseProfileDraft(readProfileDraft(id))
      const next = change(kept === null ? stored : rebaseDraft(kept, stored))
      // Kept against what is stored now, so the next read has nothing to
      // carry it across; an edit back to the stored profile keeps nothing.
      keepProfileDraft(id, sameValue(next, stored) ? null : { base: stored, config: next })
    },
    [id, stored],
  )

  const discard = useCallback(() => {
    const was = readProfileDraft(id)
    keepProfileDraft(id, null)
    return was
  }, [id])

  const restore = useCallback((kept: string) => keepProfileDraft(id, kept), [id])

  return { config, dirty, edit, discard, restore }
}
