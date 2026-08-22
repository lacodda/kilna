import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import {
  calendar as fetchCalendar,
  markReleased,
  releaseQueue,
  scheduleRelease,
  setSlotPin,
  unscheduleRelease,
  warnUnreadyReleases,
} from '@/lib/api'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { labelOf, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/Button'
import { DatePicker } from '@/components/ui/DatePicker'
import { PromptDialog } from '@/components/ui/Dialog'
import { SkeletonList } from '@/components/ui/Skeleton'
import { MonthGrid } from '@/components/calendar/MonthGrid'
import { ReadyMarks } from '@/components/calendar/ReadyMarks'
import { ReleaseEditor } from '@/components/calendar/ReleaseEditor'
import { monthOf, today, type Month } from '@/lib/month'
import { cn } from '@/lib/utils'

interface Props {
  onSelect: (workId: string) => void
}

// The queue feeds the calendar: strongest first on the left, dated slots on the
// right. Claiming a taken slot is the one place the app pushes back.
export function CalendarView({ onSelect }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const client = useQueryClient()

  const [picked, setPicked] = useState<string | null>(null)
  const [slot, setSlot] = useState('')
  const [month, setMonth] = useState<Month>(() => monthOf(today()))
  // The id of the release being edited, not the row itself: holding the row
  // would freeze it at the moment it was opened, and pinning from inside the
  // dialog left the tick unmoved until it was closed and opened again.
  const [editingId, setEditingId] = useState<string | null>(null)
  // The release waiting for a link, or null when the dialog is closed.
  const [releasing, setReleasing] = useState<string | null>(null)

  const slots = useQuery({ queryKey: keys.calendar, queryFn: fetchCalendar })
  const queued = useQuery({ queryKey: keys.releaseQueue, queryFn: releaseQueue })

  // Both sides of this screen move together: taking a slot removes something
  // from the queue, returning one puts it back. The journal goes with them —
  // displacing a release writes the one warning that lights the bell, and a
  // change that put something unready inside the coming week warns right away
  // rather than at the next startup. The sweep runs before the journal is
  // refetched so the feed the refetch brings back already holds the warning.
  const settle = () => {
    void client.invalidateQueries({ queryKey: keys.calendar })
    void client.invalidateQueries({ queryKey: keys.releaseQueue })
    void client.invalidateQueries({ queryKey: keys.releases })
    void warnUnreadyReleases(today()).finally(() => {
      void client.invalidateQueries({ queryKey: keys.journal })
    })
  }

  const claim = useMutation({
    mutationFn: ({ id, date }: { id: string; date: string }) => scheduleRelease(id, date),
    onSuccess: (result) => {
      setPicked(null)
      settle()

      if (result.displaced === null) {
        say.ok(t('toast.releaseScheduled'))
      } else {
        // Displacement is the rule working, not a failure — but it is a
        // consequence someone did not ask for, so it gets said out loud.
        say.warn(
          t('toast.releaseScheduled'),
          t('calendar.displaced', {
            title: result.displaced.title ?? result.displaced.work_id,
          }),
        )
      }
    },
    onError: (cause) => say.failedTo(t('toast.releaseSaveFailed'), cause),
  })

  const release = useMutation({
    mutationFn: ({ id, url }: { id: string; url?: string }) => markReleased(id, url),
    onSuccess: () => {
      settle()
      say.ok(t('toast.releaseReleased'))
    },
    onError: (cause) => say.failedTo(t('toast.releaseSaveFailed'), cause),
  })

  // Dragging goes through the contest, not through a plain edit. `update`
  // writes the date without looking at who holds it — verified, two releases
  // landed on the same day — and a calendar where the rule can be sidestepped
  // by dragging is a calendar without the rule.
  const move = useMutation({
    mutationFn: ({ id, date }: { id: string; date: string }) => scheduleRelease(id, date),
    onSuccess: (result) => {
      settle()
      if (result.displaced === null) {
        say.ok(t('toast.releaseMoved'))
      } else {
        say.warn(
          t('toast.releaseMoved'),
          t('calendar.displaced', {
            title: result.displaced.title ?? result.displaced.work_id,
          }),
        )
      }
    },
    onError: (cause) => say.failedTo(t('toast.releaseSaveFailed'), cause),
  })

  const pin = useMutation({
    mutationFn: ({ id, pinned }: { id: string; pinned: boolean }) => setSlotPin(id, pinned),
    onSuccess: (release) => {
      settle()
      say.ok(release.slot_pinned_at === null ? t('toast.slotUnpinned') : t('toast.slotPinned'))
    },
    onError: (cause) => say.failedTo(t('toast.releaseSaveFailed'), cause),
  })

  const unschedule = useMutation({
    mutationFn: unscheduleRelease,
    onSuccess: () => {
      settle()
      say.ok(t('toast.releaseUnscheduled'))
    },
    onError: (cause) => say.failedTo(t('toast.releaseSaveFailed'), cause),
  })

  return (
    <div className="grid gap-6 lg:grid-cols-[22rem_1fr]">
      <section className="flex flex-col gap-3">
        <h3 className="text-sm font-semibold">{t('calendar.queue')}</h3>
        <p className="text-xs text-dim">{t('calendar.queueHint')}</p>

        {queued.isPending ? (
          <SkeletonList rows={4} />
        ) : queued.isError ? (
          <p role="alert" className="text-sm text-bad">
            {t('toast.loadFailed')}
          </p>
        ) : queued.data.length === 0 ? (
          <p className="py-6 text-sm text-dim">{t('calendar.queueEmpty')}</p>
        ) : (
          <ul className="flex flex-col gap-1">
            {queued.data.map((entry) => (
              <li key={entry.id}>
                <button
                  type="button"
                  onClick={() => setPicked(entry.id === picked ? null : entry.id)}
                  className={cn(
                    'flex w-full cursor-pointer items-center gap-2 rounded-[9px] px-3 py-2 text-left text-sm transition-colors',
                    entry.id === picked ? 'bg-accent-soft text-accent-2' : 'hover:bg-soft',
                  )}
                >
                  <span className="flex-1 truncate font-medium">{entry.work_title}</span>
                  {/* No date yet, so no deadline: the gaps show, calmly. */}
                  <ReadyMarks readiness={entry.readiness} released={false} daysLeft={null} />
                  <span className="text-xs text-faint">
                    {labelOf(profile.config.release_kinds, entry.kind)}
                  </span>
                  <span
                    className="w-10 text-right font-mono tabular-nums"
                    // An unscored work cannot take a slot from a scored one,
                    // and finding that out from a refusal is late.
                    title={entry.total === null ? t('calendar.unscored') : undefined}
                  >
                    {entry.total?.toFixed(0) ?? '—'}
                  </span>
                </button>
              </li>
            ))}
          </ul>
        )}

        {picked !== null && (
          <form
            className="flex gap-2"
            onSubmit={(event) => {
              event.preventDefault()
              if (slot !== '') claim.mutate({ id: picked, date: slot })
            }}
          >
            <DatePicker
              value={slot}
              onChange={setSlot}
              placeholder={t('calendar.slotDate')}
              aria-label={t('calendar.slotDate')}
            />
            <Button type="submit" variant="primary" disabled={slot === '' || claim.isPending}>
              {t('calendar.claim')}
            </Button>
          </form>
        )}
      </section>

      <section className="flex flex-col gap-3">
        {slots.isPending ? (
          <SkeletonList rows={4} />
        ) : slots.isError ? (
          <p role="alert" className="text-sm text-bad">
            {t('toast.loadFailed')}
          </p>
        ) : (
          <>
            <MonthGrid
              month={month}
              onMonthChange={setMonth}
              slots={slots.data}
              // A queued release is waiting for a date: the grid becomes a way
              // to pick one, rather than a picture of what is booked.
              claimingId={picked}
              onPickDay={(date) => {
                if (picked !== null) claim.mutate({ id: picked, date })
              }}
              onOpenRelease={setEditingId}
              onMove={(id, date) => move.mutate({ id, date })}
              onUnschedule={(id) => unschedule.mutate(id)}
            />

            {slots.data.length === 0 && <p className="text-sm text-dim">{t('calendar.empty')}</p>}
          </>
        )}
      </section>

      {/* Found afresh on every render rather than held in state: pinning from
          inside the dialog otherwise left the tick unmoved until it was closed
          and opened again. An invalidated query keeps serving what it has while
          it refetches, so the row does not vanish out from under the dialog —
          checked by pinning with the dialog open. */}
      <ReleaseEditor
        release={slots.data?.find((entry) => entry.id === editingId) ?? null}
        onOpenChange={(open) => {
          if (!open) setEditingId(null)
        }}
        onSaved={settle}
        onOpenWork={onSelect}
        onMarkReleased={setReleasing}
        onUnschedule={(id) => unschedule.mutate(id)}
        onTogglePin={(id, pinned) => pin.mutate({ id, pinned })}
      />

      {/* Was `window.prompt()`: blocking, unstyled, and untranslatable. */}
      <PromptDialog
        open={releasing !== null}
        onOpenChange={(open) => {
          if (!open) setReleasing(null)
        }}
        title={t('calendar.markReleased')}
        label={t('calendar.urlPrompt')}
        placeholder={t('calendar.urlPrompt')}
        confirmLabel={t('calendar.markReleased')}
        // The link is optional: something can go out without one.
        allowEmpty
        onSubmit={(url) => {
          if (releasing !== null) release.mutate({ id: releasing, url: url === '' ? undefined : url })
          setReleasing(null)
        }}
      />
    </div>
  )
}
