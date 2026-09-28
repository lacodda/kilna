import type { VersionRole, VersionSummary } from '@/lib/api/types'

/**
 * What is written about a version, and what a piece of commentary is about.
 *
 * A role that names another in `comments_on` is commentary on it: a review
 * and a critique of the lyrics. Commentary is read beside the revision it
 * discusses, and a review of revision 2 says nothing about revision 5 -
 * showing it beside 5 would be the panel asserting something nobody wrote. So
 * the pairing is exact, and it runs both ways: from the text to what was said
 * about it, and from a review back to the text it read.
 *
 * Commentary that says which version it is about (`about_version_id`, written
 * by an action started on that version) is paired by that. Commentary that
 * does not - imported, or written before the field existed - is paired by
 * revision number, the way it always was.
 */

/** Whether `role` is written about another role rather than standing alone. */
export function isCommentary(roles: readonly VersionRole[], role: string): boolean {
  return roles.find((r) => r.key === role)?.comments_on !== undefined
}

/** Everything written about `open`, in the order the history lists it. */
export function commentaryOn(
  versions: readonly VersionSummary[],
  roles: readonly VersionRole[],
  open: VersionSummary | null,
): VersionSummary[] {
  if (open === null) return []
  const about = new Set(roles.filter((r) => r.comments_on === open.role).map((r) => r.key))
  return versions.filter(
    (version) =>
      about.has(version.role) &&
      (version.about_version_id !== null
        ? version.about_version_id === open.id
        : version.revision === open.revision),
  )
}

/**
 * The version a piece of commentary was written about, when it is still
 * there. Null for a version that is not commentary, and for commentary whose
 * text has been deleted or never had the revision it names.
 */
export function subjectOf(
  versions: readonly VersionSummary[],
  roles: readonly VersionRole[],
  commentary: VersionSummary | null,
): VersionSummary | null {
  if (commentary === null) return null
  const role = roles.find((r) => r.key === commentary.role)?.comments_on
  if (role === undefined) return null
  const found = versions.find((version) =>
    commentary.about_version_id !== null
      ? version.id === commentary.about_version_id
      : version.role === role && version.revision === commentary.revision,
  )
  return found ?? null
}
