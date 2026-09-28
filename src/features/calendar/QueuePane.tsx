import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { UseQueryResult } from '@tanstack/react-query'
import type { ScheduledRelease } from '@/lib/api/types'
import { coverImageFor } from '@/lib/cover'
import { formatNumber } from '@/lib/format'
import { stagesOf } from '@/lib/stages'
import { useCovers } from '@/lib/useCovers'
import { allOf, labelOf, say as sayLabel, useProfile } from '@/lib/useProfile'
import { Select } from '@/components/AppSelect'
import { DatePicker } from '@/components/DatePicker'
import { Loaded } from '@/components/Loaded'
import { ReadyMarks } from '@/components/ReadyMarks'
import { Pane } from '@/components/frame'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { Input } from '@/components/ui/input'
import { RowButton } from '@/components/ui/list-row'
import { SkeletonList } from '@/components/ui/skeleton'
import { QueueDrop } from '@/features/calendar/QueueDrop'
import type { DropTarget } from '@/features/calendar/useCalendarDrag'
import { cn } from '@/lib/utils'

interface Props {
  queued: UseQueryResult<ScheduledRelease[]>
  /** The queued release waiting for a date, if any. */
  picked: string | null
  onPick: (releaseId: string | null) => void
  /** Give the picked release the date typed in the foot. */
  onClaim: (releaseId: string, date: string) => void
  claiming: boolean
  /** Plan the whole queue to the profile's rhythm; absent while a plan is up. */
  onLayOut?: () => void
  layingOut: boolean
  /** A booked release is in the air: the queue takes it back. */
  carrying: boolean
  /** It is over the queue right now. */
  over: boolean
  target: DropTarget
}

/**
 * What still needs a date, beside the month it is read against.
 *
 * Strongest first, as the queue has always been ordered. Its head - how to
 * find one of a few hundred - and its foot stand; the list between them
 * scrolls. The foot is where a booked release comes back: the whole pane
 * takes a chip let go over it, and the foot says so before anything is in
 * the air.
 */
export function QueuePane({
  queued,
  picked,
  onPick,
  onClaim,
  claiming,
  onLayOut,
  layingOut,
  carrying,
  over,
  target,
}: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const covers = useCovers()
  const releaseKinds = allOf(profile.config, 'release_kinds')
  const tiers = allOf(profile.config, 'tiers')

  // The date typed for the picked release, when the day is not simply clicked.
  const [slot, setSlot] = useState('')
  // What the queue is narrowed to. A view of the QUEUE, not of the month: the
  // owner asked to see "what is closest to going out" among four hundred
  // works, and that question is asked of the queue. The month's kind chips
  // deliberately do not reach it - the queue is what still needs a date, and
  // hiding part of it behind a view of the month would hide work waiting.
  const [query, setQuery] = useState('')
  const [workKind, setWorkKind] = useState<string | null>(null)
  const [stage, setStage] = useState<number | null>(null)

  // The queue, as the filters leave it. `work_stage` is a percentage and the
  // filter is a stop, so the comparison is "has reached this stop" rather than
  // equality - picking "Polishing" should show what is polishing AND what is
  // past it, which is what "closest to going out" means.
  const shown = (queued.data ?? []).filter(
    (entry) =>
      (workKind === null || entry.work_kind === workKind) &&
      (stage === null || (entry.work_stage ?? -1) >= stage) &&
      entry.work_title.toLowerCase().includes(query.trim().toLowerCase()),
  )

  // One click plans the whole queue to the profile's rhythm. Without a rhythm
  // there is nothing to pace by, and the button says so instead of hiding.
  const noRhythm = profile.config.rhythm == null
  const canLayOut = onLayOut !== undefined && (queued.data?.length ?? 0) > 0

  return (
    <div
      // The whole pane takes a release back, not only its foot: a target the
      // width of a list is one a carried chip cannot miss.
      data-queue-drop
      onPointerEnter={carrying ? target.onPointerEnter : undefined}
      onPointerLeave={carrying ? target.onPointerLeave : undefined}
      className="flex min-h-0 min-w-0 flex-1 flex-col"
    >
      <Pane
        label={t('calendar.queue')}
        bodyClassName="p-1.5"
        className={cn('transition-colors', carrying && 'border-accent', over && 'bg-accent-soft')}
        head={
          <div className="flex w-full flex-col gap-2">
            <div className="flex items-center gap-2">
              <div className="min-w-0 flex-1">
                <h2 className="caption">{t('calendar.queue')}</h2>
                <p className="mt-0.5 text-xs text-faint">{t('calendar.queueHint')}</p>
              </div>
              {canLayOut && (
                <Button
                  size="sm"
                  variant="soft"
                  disabled={noRhythm || layingOut}
                  disabledReason={noRhythm ? t('calendar.layoutNeedsRhythm') : undefined}
                  onClick={onLayOut}
                >
                  {t('calendar.layout')}
                </Button>
              )}
            </div>

            {/* Finding one of a few hundred, and seeing what is nearly ready.
                The queue is ordered by score, which answers "which is best" -
                not "which is closest to going out", which is what someone
                filling a week is actually asking. */}
            <Input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder={t('calendar.queueSearch')}
              aria-label={t('calendar.queueSearch')}
              className="text-xs"
            />
            <div className="flex gap-1.5">
              <Select
                value={workKind ?? ''}
                onChange={(value) => setWorkKind(value === '' ? null : value)}
                placeholder={t('calendar.queueAnyKind')}
                aria-label={t('calendar.queueKind')}
                className="min-w-0 flex-1"
                options={profile.config.work_kinds.map((entry) => ({
                  value: entry.key,
                  label: sayLabel(entry.label),
                }))}
              />
              <Select
                value={stage === null ? '' : String(stage)}
                onChange={(value) => setStage(value === '' ? null : Number(value))}
                placeholder={t('calendar.queueAnyStage')}
                aria-label={t('calendar.queueStage')}
                className="min-w-0 flex-1"
                options={stagesOf(profile.config).map((stop) => ({
                  value: String(stop.percent),
                  label: sayLabel(stop.label),
                }))}
              />
            </div>
          </div>
        }
        foot={
          // A pick waiting for a typed date takes the foot; a chip in the air
          // takes it back, since that is what the foot is for.
          picked !== null && !carrying ? (
            <form
              className="flex w-full gap-2"
              onSubmit={(event) => {
                event.preventDefault()
                if (slot !== '') onClaim(picked, slot)
              }}
            >
              <DatePicker
                value={slot}
                onChange={setSlot}
                placeholder={t('calendar.slotDate')}
                aria-label={t('calendar.slotDate')}
              />
              <Button type="submit" variant="primary" disabled={slot === '' || claiming}>
                {t('calendar.claim')}
              </Button>
            </form>
          ) : (
            <QueueDrop active={carrying} over={over} />
          )
        }
      >
        <Loaded
          query={queued}
          skeleton={<SkeletonList rows={4} />}
          isEmpty={(data) => data.length === 0}
          emptyState={<EmptyState plain title={t('calendar.queueEmpty')} className="p-2" />}
          plain
        >
          {() =>
            shown.length === 0 ? (
              // Plenty waiting, none of it matching: the way out is the search
              // and the two filters right above.
              <EmptyState
                plain
                variant="filtered"
                title={t('calendar.queueNoMatch')}
                className="p-2"
              />
            ) : (
              <ul className="flex flex-col gap-0.5">
                {shown.map((entry) => (
                  <li key={entry.id}>
                    <RowButton
                      selected={entry.id === picked}
                      onClick={() => onPick(entry.id === picked ? null : entry.id)}
                      className="px-2"
                      // The work's cover, as on the month: the colour a
                      // release has in the queue is the colour its chip will
                      // have once it has a day.
                      start={
                        <span
                          aria-hidden
                          className="size-6.5 rounded-sm"
                          style={{
                            background: coverImageFor(entry.work_id, covers.get(entry.work_id)),
                          }}
                        />
                      }
                      // What goes out, and how the work was judged - the
                      // mockup's line under the title.
                      description={[
                        labelOf(releaseKinds, entry.kind),
                        entry.tier === null ? null : labelOf(tiers, entry.tier),
                      ]
                        .filter((part) => part !== null)
                        .join(' · ')}
                      end={
                        <>
                          {/* No date yet, so no deadline: the gaps show, calmly. */}
                          <ReadyMarks
                            readiness={entry.readiness}
                            released={false}
                            daysLeft={null}
                          />
                          <span
                            className="w-8 text-right font-mono text-sm text-dim"
                            // The score orders the queue and drives the
                            // auto-layout; since v0.44 it decides nothing
                            // about who may have a day.
                            title={entry.total === null ? t('calendar.unscored') : undefined}
                          >
                            {entry.total === null ? '—' : formatNumber(entry.total, 0)}
                          </span>
                        </>
                      }
                    >
                      {entry.work_title}
                    </RowButton>
                  </li>
                ))}
              </ul>
            )
          }
        </Loaded>
      </Pane>
    </div>
  )
}
