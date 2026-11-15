import { useQuery } from '@tanstack/react-query'
import type { SlotPreview } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { useRested } from '@/lib/useRested'
import { landingOf } from '@/features/calendar/useCalendarDrag'

interface Options {
  /** What is in the air or waiting for a day: a release's id, or a work's. */
  moving: string | null
  /** The day under the pointer, if it is one of the days on screen. */
  over: string | null
}

/**
 * What the day under a carried release would say before it lands: who is
 * already there, whether that date was kept, what it would repeat (ADR 0054),
 * whether the day is closed (v0.96). Nothing here refuses; a day holds what is
 * put on it.
 *
 * The backend is asked, for a release and for a work alike: it knows the song
 * either is made from. A work from the pane has no release to ask with, and
 * until v0.111 its day was read off the screen instead - who is there, and
 * nothing of what it would repeat.
 *
 * A day is asked about once the pointer has rested on it (`useRested`): the
 * days a chip crosses on its way are not. One already asked about during this
 * carry answers at once, from what was heard.
 */
export function useLandingVerdict({ moving, over }: Options) {
  const rested = useRested(over)
  const asked = useQuery({
    ...queries.slotPreview(landingOf(moving ?? ''), over ?? ''),
    enabled: moving !== null && over !== null && rested,
    staleTime: 5_000,
  })

  /** The verdict for `date`, when the carried thing is over it and the day has
   *  anything to say. */
  return (date: string): SlotPreview | null => {
    if (over !== date || moving === null) return null
    const preview = asked.data
    if (preview === undefined) return null
    const says =
      preview.verdict !== 'empty' || preview.repeats.length > 0 || preview.closed !== null
    return says ? preview : null
  }
}
