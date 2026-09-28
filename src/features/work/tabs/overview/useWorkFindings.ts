import { useQuery } from '@tanstack/react-query'
import { findings, visible, type Finding } from '@/lib/findings'
import { today } from '@/lib/month'
import { queries } from '@/lib/query/queries'
import { useProfile } from '@/lib/useProfile'

/**
 * What the workspace notices about this one work and nobody has put away yet.
 *
 * The dashboard's findings (`lib/findings`), read from the same three answers
 * - the catalogue, the calendar, what was put away - and narrowed to the work:
 * one reckoning for both screens, so a complaint heard on the dashboard is
 * quiet here too. Nothing until all three have arrived, rather than a
 * complaint that flashes up and is taken back when the dismissals land.
 */
export function useWorkFindings(workId: string): Finding[] {
  const profile = useProfile()
  const works = useQuery(queries.catalogue())
  const slots = useQuery(queries.calendar())
  const dismissed = useQuery(queries.dismissals())

  if (works.data === undefined || slots.data === undefined || dismissed.data === undefined) {
    return []
  }
  // The user's own day: the backend knows only UTC, and after sunset at a
  // negative offset that is already tomorrow.
  const standing = findings(works.data, slots.data, profile.config, today())
  return visible(standing, dismissed.data).filter((finding) => finding.workId === workId)
}
