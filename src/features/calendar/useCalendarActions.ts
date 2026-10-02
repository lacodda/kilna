import { useTranslation } from 'react-i18next'
import { useQueryClient } from '@tanstack/react-query'
import {
  applyLayout,
  createRelease,
  generateReleaseFieldsBatch,
  markReleased,
  planLayout,
  scheduleRelease,
  unscheduleRelease,
  warnUnreadyReleases,
} from '@/lib/api/releases'
import type { NewRelease, Placement, RepeatFinding } from '@/lib/api/types'
import { today } from '@/lib/month'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { landingWarning } from '@/lib/repeats'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { say as sayLabel } from '@/lib/useProfile'

/**
 * Both sides of this screen move together: taking a slot removes something
 * from the queue, returning one puts it back - what every one of these writes
 * disturbs. And the guard of repeats (ADR 0054), which reads the days songs
 * go out on: a release moved out of a song's window turns its red to orange.
 */
const REFRESHED = [keys.calendar, keys.releaseQueue, keys.releases, keys.register] as const

interface Options {
  /** The calendar changed: a previewed plan is now a picture of the past. */
  onChanged: () => void
  /** The auto-layout came back with a plan to approve. */
  onPlanned: (placements: Placement[]) => void
  /** A queued release was given a date. */
  onClaimed: () => void
  /** A work chosen for a day went onto it. */
  onDayFilled: () => void
}

/**
 * Every write the calendar makes.
 *
 * Held apart from the screen that draws them because they are the one part of
 * it that is not layout: the writes, each refreshing both sides and each
 * settling the same way. The screen hands in what its own state does when
 * one lands. Editing a booking is not among them: the release's dialog saves
 * its own form (`ReleaseEditor`), the one the Releases tab saves too.
 */
export function useCalendarActions({ onChanged, onPlanned, onClaimed, onDayFilled }: Options) {
  const { t } = useTranslation()
  const client = useQueryClient()

  // What settling still does beyond the plain refresh: clear the preview, and
  // warn about the week ahead before the journal is read again. Displacing a
  // release writes the one warning that lights the bell, and a change that put
  // something unready inside the coming week warns right away rather than at
  // the next startup - but only once the sweep itself has written it, which is
  // later than the hook's own immediate refresh of the journal. So the feed is
  // asked again once the sweep is done, and holds the warning that arrived too
  // late for the first read.
  const settle = () => {
    onChanged()
    void warnUnreadyReleases(today()).finally(() => {
      void client.invalidateQueries({ queryKey: keys.journal })
    })
  }

  /**
   * What a release would repeat on `date` (ADR 0054), asked before it is put
   * there: the dry run the month ran while the chip hovered the day, read
   * from the cache when it is that fresh. Never in the way of the write - a
   * day it could not be asked about is a day it lands on without a word.
   */
  const repeatsOn = async (id: string, date: string): Promise<RepeatFinding[]> => {
    try {
      const preview = await client.fetchQuery({
        ...queries.slotPreview(id, date),
        staleTime: 5_000,
      })
      return preview.repeats
    } catch {
      return []
    }
  }

  /** A release placed: said plainly, or - when it repeats a song out or
   *  booked near that day - as a warning naming the word, the song and its
   *  day. It is placed either way; the guard warns and does not refuse. */
  const landed = (message: string, repeats: readonly RepeatFinding[]) => {
    const warning = landingWarning(repeats)
    if (warning === null) say.ok(message)
    else say.warn(message, warning)
  }

  const claim = useAppMutation({
    mutationFn: async ({ id, date }: { id: string; date: string }) => {
      const repeats = await repeatsOn(id, date)
      await scheduleRelease(id, date)
      return repeats
    },
    failure: 'toast.releaseSaveFailed',
    refresh: REFRESHED,
    onSuccess: (repeats) => {
      onClaimed()
      settle()
      landed(t('toast.releaseScheduled'), repeats)
    },
  })

  // A work chosen for a day, booked in one step: the release is made already
  // holding its date, rather than being made and then dragged out of the
  // queue onto the day it was asked for a moment ago.
  const fillDay = useAppMutation({
    mutationFn: (release: NewRelease) => createRelease(release),
    failure: 'toast.releaseSaveFailed',
    refresh: REFRESHED,
    onSuccess: () => {
      onDayFilled()
      settle()
      say.ok(t('toast.releaseScheduled'))
    },
  })

  const release = useAppMutation({
    mutationFn: ({ id, url, at }: { id: string; url: string | null; at: string | null }) =>
      markReleased(id, url, at),
    failure: 'toast.releaseSaveFailed',
    refresh: REFRESHED,
    onSuccess: () => {
      settle()
      say.ok(t('toast.releaseReleased'))
    },
  })

  // The same call the queue uses. Until v0.44 dragging went through a contest
  // and a weaker release could be evicted by the drop; now a day holds what is
  // put on it, so moving a chip is the plainest thing on the screen - a date
  // is written, and nothing else happens. What it would repeat there is
  // asked first and said with the toast (ADR 0054); it refuses nothing.
  const move = useAppMutation({
    mutationFn: async ({ id, date }: { id: string; date: string }) => {
      const repeats = await repeatsOn(id, date)
      await scheduleRelease(id, date)
      return repeats
    },
    failure: 'toast.releaseSaveFailed',
    refresh: REFRESHED,
    onSuccess: (repeats) => {
      settle()
      landed(t('toast.releaseMoved'), repeats)
    },
  })

  const unschedule = useAppMutation({
    mutationFn: unscheduleRelease,
    failure: 'toast.releaseSaveFailed',
    refresh: REFRESHED,
    onSuccess: () => {
      settle()
      say.ok(t('toast.releaseUnscheduled'))
    },
  })

  // The plan moves nothing; it is a picture to approve.
  const preview = useAppMutation({
    mutationFn: () => planLayout(today()),
    failure: 'toast.layoutFailed',
    onSuccess: onPlanned,
  })

  // What the month's releases go out as, written in one pass. The point is a
  // week of the calendar: someone who planned six videos writes their
  // metadata together or not at all.
  const fillFields = useAppMutation({
    mutationFn: (ids: string[]) => generateReleaseFieldsBatch(ids),
    failure: 'calendar.fields.failed',
    refresh: [keys.releases],
    onSuccess: (outcome) => {
      if (outcome.filled > 0) {
        say.ok(t('calendar.fields.done', { count: outcome.filled }))
      } else {
        say.ok(t('calendar.fields.doneNone'))
      }
      say.skipped(outcome.skipped)
      // One line per refusal, because each is a different work waiting on a
      // different thing, and a single line holding six of them is read by
      // nobody.
      for (const refusal of outcome.refused) {
        say.warn(
          t('calendar.fields.refusedRow', {
            title: refusal.workTitle,
            // A shipped profile names its fields in both languages.
            label: sayLabel(refusal.label),
            reason: refusal.reason,
          }),
        )
      }
    },
  })

  const book = useAppMutation({
    mutationFn: (placements: Placement[]) => applyLayout(placements),
    refresh: REFRESHED,
    onSuccess: () => {
      settle()
      say.ok(t('toast.layoutApplied'))
    },
    // A stale plan is refused whole; the refetch shows what the calendar
    // actually holds now, and the person previews again from that. `refresh`
    // only runs on success, so the same areas are invalidated by hand here.
    onError: (cause) => {
      for (const key of REFRESHED) void client.invalidateQueries({ queryKey: key })
      settle()
      say.failedTo(t('toast.layoutFailed'), cause)
    },
  })

  return { settle, claim, fillDay, release, move, unschedule, preview, fillFields, book }
}

export type CalendarActions = ReturnType<typeof useCalendarActions>
