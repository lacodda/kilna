/**
 * Unsaved version text, kept across restarts.
 *
 * A draft is not a version yet — it has no revision, no history and nothing
 * points at it — so it does not belong in the database. But losing a page of
 * writing because a window was closed is the kind of thing a writing tool is
 * never forgiven for, so it is not kept only in memory either.
 *
 * `localStorage` is the middle: survives a restart, costs no schema, and is
 * cleared the moment the text becomes a real version.
 */

const PREFIX = 'kilna.draft'

/** One slot per work and role: two roles are two independent drafts. */
function slot(workId: string, role: string): string {
  return `${PREFIX}.${workId}.${role}`
}

export function readDraft(workId: string, role: string): string {
  try {
    return localStorage.getItem(slot(workId, role)) ?? ''
  } catch {
    // Storage can be unavailable or full; a lost draft is bad, a crashed
    // editor is worse.
    return ''
  }
}

export function writeDraft(workId: string, role: string, body: string): void {
  try {
    if (body === '') localStorage.removeItem(slot(workId, role))
    else localStorage.setItem(slot(workId, role), body)
  } catch {
    // Nothing to do about a full disk here; the text is still on screen.
  }
}

export function clearDraft(workId: string, role: string): void {
  writeDraft(workId, role, '')
}

/**
 * Drop every draft belonging to a work.
 *
 * Called when the work itself is deleted for good, so the trash does not leave
 * orphan text behind that nothing can ever reach.
 */
export function clearDraftsFor(workId: string): void {
  try {
    const doomed = Object.keys(localStorage).filter((key) =>
      key.startsWith(`${PREFIX}.${workId}.`),
    )
    for (const key of doomed) localStorage.removeItem(key)
  } catch {
    // Same reasoning as above.
  }
}
