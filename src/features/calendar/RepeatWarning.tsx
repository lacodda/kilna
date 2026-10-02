import { useQuery } from '@tanstack/react-query'
import { queries } from '@/lib/query/queries'
import { landingWarning } from '@/lib/repeats'
import { cn } from '@/lib/utils'

interface Props {
  releaseId: string
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
 * chip is carried (`preview_schedule`), so the dialog and the month cannot
 * disagree about the same day.
 */
export function RepeatWarning({ releaseId, day, className }: Props) {
  const preview = useQuery({
    ...queries.slotPreview(releaseId, day),
    enabled: day !== '',
    staleTime: 5_000,
  })
  const said = preview.data === undefined ? null : landingWarning(preview.data.repeats)
  if (said === null) return null
  return (
    // `status`: the line appears as a date is picked, and a reader who picked
    // it by keyboard hears it without going to look.
    <p role="status" className={cn('text-xs text-bad', className)}>
      {said}
    </p>
  )
}
