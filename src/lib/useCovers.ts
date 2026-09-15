import { useQuery } from '@tanstack/react-query'
import { fileSrc, listCovers } from '@/lib/api'
import { keys } from '@/lib/query'

/**
 * The cover of every work that has one, as a URL the window may fetch.
 *
 * One query for the whole profile rather than one per work: the catalogue
 * draws two hundred rows, the calendar a month of chips, and asking per row
 * is the shape that made the predecessor's screens slow. A work with no
 * cover is simply absent from the map, and the gradient stands in.
 */
export function useCovers(): Map<string, string> {
  const covers = useQuery({
    queryKey: keys.covers,
    queryFn: listCovers,
  })

  const found = new Map<string, string>()
  for (const [workId, path] of covers.data ?? []) found.set(workId, fileSrc(path))
  return found
}
