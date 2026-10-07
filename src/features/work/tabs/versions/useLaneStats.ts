import { useMemo } from 'react'
import { useQueries, type UseQueryResult } from '@tanstack/react-query'
import type { Version, VersionSummary } from '@/lib/api/types'
import { predecessor } from '@/lib/history'
import { queries } from '@/lib/query/queries'
import { statsOf, type VersionStats } from '@/lib/versionStats'

/** The bodies that have come, by version id: stable while none changes. */
function bodiesOf(results: UseQueryResult<Version | null>[]): Map<string, string> {
  const bodies = new Map<string, string>()
  for (const result of results) {
    if (result.data != null) bodies.set(result.data.id, result.data.body)
  }
  return bodies
}

/**
 * What each row of a role's list says about its text (`lib/versionStats`):
 * words, lines, and the lines in and out since the version it is compared
 * with by default.
 *
 * Read off the bodies, which the list itself does not carry: one request per
 * version of the role, the same ones opening a version asks for, so opening
 * a row finds its text already here. The change is counted only where it
 * means something - between drafts, not between reviews of different
 * revisions - and only once both bodies have come.
 */
export function useLaneStats(
  lane: readonly VersionSummary[],
  compared: boolean,
): Record<string, VersionStats> {
  const bodies = useQueries({
    queries: lane.map((version) => queries.version(version.id)),
    combine: bodiesOf,
  })

  return useMemo(() => {
    const out: Record<string, VersionStats> = {}
    for (const version of lane) {
      const body = bodies.get(version.id)
      if (body === undefined) continue
      const before = compared ? predecessor(lane, version.id) : null
      const previous = before === null ? null : (bodies.get(before.id) ?? null)
      out[version.id] = statsOf(body, previous)
    }
    return out
  }, [lane, bodies, compared])
}
