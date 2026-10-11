import { useQuery } from '@tanstack/react-query'
import type { ScheduledRelease, SlotPreview, TimeOff } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { carriedWork } from '@/features/calendar/useCalendarDrag'

interface Options {
  /** What is in the air or waiting for a day: a release's id, or a work's. */
  moving: string | null
  /** The day under the pointer, if it is one of the days on screen. */
  over: string | null
  slots: readonly ScheduledRelease[]
  closed: ReadonlyMap<string, TimeOff>
}

/**
 * What the day under a carried release would say before it lands: who is
 * already there, whether that date was kept, what it would repeat (ADR 0054),
 * whether the day is closed (v0.96). Nothing here refuses; a day holds what is
 * put on it.
 *
 * A release is asked about by the backend, which knows its song. A work from
 * the pane has no release yet to ask with, so its day is read off what the
 * screen already holds - who is there and whether the day is closed - and the
 * repeats are said when it lands, as a work booked from a day's `+` says them.
 */
export function useLandingVerdict({ moving, over, slots, closed }: Options) {
  const work = moving === null ? null : carriedWork(moving)
  const asked = useQuery({
    ...queries.slotPreview(moving as string, over as string),
    enabled: moving !== null && work === null && over !== null,
    staleTime: 5_000,
  })

  const read = (date: string): SlotPreview | null => {
    if (work !== null) return onScreen(date, slots, closed)
    return asked.data ?? null
  }

  /** The verdict for `date`, when the carried thing is over it and the day has
   *  anything to say. */
  return (date: string): SlotPreview | null => {
    if (over !== date || moving === null) return null
    const preview = read(date)
    if (preview === null) return null
    const says =
      preview.verdict !== 'empty' || preview.repeats.length > 0 || preview.closed !== null
    return says ? preview : null
  }
}

/** A day's verdict from what the screen holds: the backend's rule, read the
 *  same way - planned releases only, a kept one first. */
function onScreen(
  date: string,
  slots: readonly ScheduledRelease[],
  closed: ReadonlyMap<string, TimeOff>,
): SlotPreview {
  const there = slots.filter((slot) => slot.scheduled_at === date && slot.status === 'planned')
  const kept = there.find((slot) => slot.slot_pinned_at !== null)
  const holder = kept ?? there[0]
  return {
    verdict: kept !== undefined ? 'pinned' : holder !== undefined ? 'taken' : 'empty',
    holder_title: holder?.work_title ?? null,
    repeats: [],
    closed: closed.get(date)?.label ?? null,
  }
}
