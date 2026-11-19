import { useQuery } from '@tanstack/react-query'
import { activeTaskTimes } from '@/lib/api/assistant'
import { keys } from '@/lib/query/keys'

/**
 * When each running task began, by its key, as the backend's runs keep it
 * (v0.163) - so a clock beside a run counts from its start, not from when
 * this screen first saw it going. Asked again whenever the running set
 * changes: a run that just began has a start the last answer did not hold.
 */
export function useTaskTimes(running: ReadonlySet<string>): (key: string) => number | null {
  const times = useQuery({
    queryKey: [...keys.activeTaskTimes, [...running].sort().join(' ')],
    queryFn: activeTaskTimes,
    enabled: running.size > 0,
  })
  return (key) => {
    const written = times.data?.[key]
    if (written === undefined) return null
    const at = Date.parse(written)
    return Number.isNaN(at) ? null : at
  }
}
