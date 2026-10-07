/**
 * Moving through a work's history.
 *
 * Versions are immutable - `update_version` does not exist and will not, since
 * merging two devices in a later major rests on a revision never changing
 * under it. Editing therefore means *starting the next revision from one*,
 * and a revision remembers which one it was written from (ADR 0055): the
 * history of a role is a tree drawn over a line of revision numbers. Reading
 * means stepping along the line, or up and down the tree, rather than hunting
 * in the list. All of it is small enough to be arithmetic, and arithmetic is
 * worth testing.
 */

/** Enough of a version summary to step through history. */
export interface Step {
  id: string
}

/** Enough of a version summary to know where it came from. */
export interface Lineage extends Step {
  revision: number
  /** The version it was written from, when that was recorded. */
  parent_version_id: string | null
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
  versions: readonly T[],
  openId: string | null,
  direction: -1 | 1,
): T | null {
  if (openId === null) return null
  const at = versions.findIndex((version) => version.id === openId)
  if (at === -1) return null
  return versions[at + direction] ?? null
}

/** The version the open one was written from, when it is in `versions`. */
export function parentIn<T extends Lineage>(
  versions: readonly T[],
  openId: string | null,
): T | null {
  const open = versions.find((version) => version.id === openId)
  if (open?.parent_version_id == null) return null
  return versions.find((version) => version.id === open.parent_version_id) ?? null
}

/** The versions written from the open one, newest first. */
export function childrenIn<T extends Lineage>(versions: readonly T[], openId: string | null): T[] {
  if (openId === null) return []
  return versions
    .filter((version) => version.parent_version_id === openId)
    .sort((a, b) => b.revision - a.revision)
}

/**
 * What the open version is compared against by default - in the list's
 * figures, in the marks on its text, in the one-press comparison.
 *
 * The version it was written from, when that was recorded and is still in
 * the role: a revision started from v3 is about what changed since v3, not
 * since whatever happened to be numbered just below it. Otherwise the
 * revision before it, which is what a history with no recorded lineage has
 * always meant. The oldest revision of a role has nothing before it, and
 * saying so is more honest than comparing it with itself.
 */
export function predecessor<T extends Lineage>(
  versions: readonly T[],
  openId: string | null,
): T | null {
  const open = versions.find((version) => version.id === openId)
  if (open === undefined) return null
  const parent = parentIn(versions, openId)
  if (parent !== null) return parent
  let before: T | null = null
  for (const version of versions) {
    if (version.revision < open.revision && (before === null || version.revision > before.revision))
      before = version
  }
  return before
}
