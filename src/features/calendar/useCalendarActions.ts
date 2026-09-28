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
import type { NewRelease, Placement } from '@/lib/api/types'
import { today } from '@/lib/month'
import { keys } from '@/lib/query/keys'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { say as sayLabel } from '@/lib/useProfile'

/**
 * Both sides of this screen move together: taking a slot removes something
 * from the queue, returning one puts it back - what every one of these writes
 * disturbs.
 */
const REFRESHED = [keys.calendar, keys.releaseQueue, keys.releases] as const

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

  const claim = useAppMutation({
    mutationFn: ({ id, date }: { id: string; date: string }) => scheduleRelease(id, date),
    failure: 'toast.releaseSaveFailed',
    refresh: REFRESHED,
    onSuccess: () => {
      onClaimed()
      settle()
      say.ok(t('toast.releaseScheduled'))
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
  // is written, and nothing else happens.
  const move = useAppMutation({
    mutationFn: ({ id, date }: { id: string; date: string }) => scheduleRelease(id, date),
    failure: 'toast.releaseSaveFailed',
    refresh: REFRESHED,
    onSuccess: () => {
      settle()
      say.ok(t('toast.releaseMoved'))
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
