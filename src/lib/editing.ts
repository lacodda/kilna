/**
 * The editing session: which version a change belongs in.
 *
 * A version used to be immutable and every revision a deliberate save. Now
 * clicking into the text and typing is the revision: the first change mints
 * the next version and every change after it goes into that same version,
 * saved as you go. What makes this one revision rather than fifty is the
 * session — one per role, holding the version it minted and when it was last
 * typed into. A change belongs in the session's version while the text open
 * on screen *is* that version and the person has not walked away from it.
 *
 * Two things end a session. Leaving the text: the panel unmounts, another
 * version or role is opened — the next change starts from whatever is open
 * then, as its own revision. And a pause: an evening's break is a new
 * sitting, and the next morning's edits should not disappear into
 * yesterday's revision. The owner's rule, 2026-09-10: one version per
 * sitting, not one per opening and not one per N minutes.
 */

export interface Session {
  /** The version the session minted, and keeps writing into. */
  versionId: string
  role: string
  /** When it was last typed into, in epoch milliseconds. */
  lastEditAt: number
}

/** How long without a keystroke turns the next one into a new sitting. */
export const SESSION_PAUSE_MS = 20 * 60 * 1000

/**
 * Whether a change to `openId`, in `role`, at `now`, belongs in the session's
 * version — or has to start the next revision.
 */
export function continues(
  session: Session | null,
  role: string,
  openId: string | null,
  now: number,
): boolean {
  if (session === null || openId === null) return false
  if (session.role !== role || session.versionId !== openId) return false
  return now - session.lastEditAt <= SESSION_PAUSE_MS
}

/** The session, having just been typed into. */
export function touched(session: Session, now: number): Session {
  return { ...session, lastEditAt: now }
}

/** A session that begins with the version just minted. */
export function begun(versionId: string, role: string, now: number): Session {
  return { versionId, role, lastEditAt: now }
}
