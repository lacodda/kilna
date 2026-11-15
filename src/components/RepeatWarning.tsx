import { useQuery } from '@tanstack/react-query'
import type { Landing } from '@/lib/api/releases'
import { queries } from '@/lib/query/queries'
import { landingWarning } from '@/lib/repeats'
import { useRested } from '@/lib/useRested'
import { cn } from '@/lib/utils'

interface Props {
  /** What would land: a release, or a work that has none yet. Nothing is
   *  asked while there is none. */
  landing: Landing | null
  /** The day being considered, `yyyy-mm-dd`; nothing is asked while empty. */
  day: string
  className?: string
}

/**
 * What a release would repeat if it went out on `day`, said before it is
 * given the day (ADR 0054): the rare word its song shares with a song out or
 * booked within the guard's window of that day, the other song, and when.
 *
 * A warning, never a refusal - the same footing as a day that is taken or
 * kept: the person may still put it there, and the calendar says why it
 * might not be wise. Asked through the same dry run the month uses while a
 * chip is carried, so a dialog and the month cannot disagree about the same
 * day - and once what is asked about has rested, so a list walked with the
 * arrows asks about the row it stops on, not every row on the way.
 */
export function RepeatWarning({ landing, day, className }: Props) {
  const id = landing === null ? null : 'work' in landing ? landing.work : landing.release
  const rested = useRested(`${id ?? ''}:${day}`)
  const preview = useQuery({
    ...queries.slotPreview(landing ?? { release: '' }, day),
    enabled: landing !== null && day !== '' && rested,
    staleTime: 5_000,
  })
  const said =
    landing === null || preview.data === undefined ? null : landingWarning(preview.data.repeats)
  if (said === null) return null
  return (
    // `status`: the line appears as a date is picked, and a reader who picked
    // it by keyboard hears it without going to look.
    <p role="status" className={cn('text-xs text-bad', className)}>
      {said}
    </p>
  )
}
