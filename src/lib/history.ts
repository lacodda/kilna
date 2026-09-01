/**
 * Moving through a work's history.
 *
 * Versions are immutable — `update_version` does not exist and will not, since
 * merging two devices in a later major rests on a revision never changing
 * under it. Editing therefore means *starting the next revision from this
 * one*, and reading means stepping along the list rather than hunting in it.
 * Both are small enough to be arithmetic, and arithmetic is worth testing.
 */

/** Enough of a version summary to step through history. */
export interface Step {
  id: string
}

/**
 * The neighbour one step away in a newest-first list.
 *
 * `-1` walks toward the newer end, `+1` toward the older, matching what the
 * arrow keys do on the list as drawn. Returns `null` at either end rather than
 * wrapping: a history has a first and a last revision, and pretending it is a
 * ring loses that.
 */
export function neighbour<T extends Step>(
  versions: T[],
  openId: string | null,
  direction: -1 | 1,
): T | null {
  if (openId === null) return null
  const at = versions.findIndex((version) => version.id === openId)
  if (at === -1) return null
  return versions[at + direction] ?? null
}

/**
 * What the open version is compared against by default.
 *
 * The previous revision of the same role — the one directly underneath it in
 * the newest-first list. The oldest revision has nothing before it, and saying
 * so is more honest than comparing it with itself.
 */
export function predecessor<T extends Step>(versions: T[], openId: string | null): T | null {
  return neighbour(versions, openId, 1)
}
