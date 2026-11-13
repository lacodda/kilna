import { useTranslation } from 'react-i18next'
import { Lock } from 'lucide-react'
import type { ScheduledRelease } from '@/lib/api/types'
import type { Ghost } from '@/lib/layout'
import { accentFor, coverImageFor } from '@/lib/cover'
import { useCovers } from '@/lib/useCovers'
import { KindGlyph } from '@/lib/releaseIcon'
import { daysBetween, missing, sayNeed } from '@/lib/readiness'
import { openExternal, shortLink } from '@/lib/link'
import { allOf, labelOf, say as sayLabel, useProfile } from '@/lib/useProfile'
import { StageDial } from '@/components/StageDial'
import { stageAt } from '@/lib/stages'
import { BigCalendarItem } from '@/components/ui/big-calendar-item'
import { Button } from '@/components/ui/button'
import { PreviewCard, PreviewCardPopup, PreviewCardTrigger } from '@/components/ui/preview-card'
import { RepeatMark } from '@/components/RepeatMark'
import { ReadyMark } from '@/features/calendar/ReadyMark'
import { cn } from '@/lib/utils'
import { formatNumber } from '@/lib/format'
import { markLabel, useRepeatMark } from '@/lib/repeats'
import { zoneLabel } from '@/lib/zonedTime'

interface Props {
  slot: ScheduledRelease
  /** The day the line sits on, for how urgent its gaps are. */
  date: string
  /** Today, so urgency is measured from one clock rather than each line's own. */
  now: string
  dragging: boolean
  /** A press landed on the line; the hook decides whether it becomes a drag. */
  onGrab: (event: React.PointerEvent) => void
  onOpen: () => void
  /** Drawn as the thing under the pointer rather than as a line on a day. */
  carried?: boolean
}

/**
 * One booked release on a day, in one line: dowel's BigCalendarItem.
 *
 * The line is the work's own colour, as a soft fill with ink of the same hue
 * (decision 01.09, kept on 24.09): the same song is the same colour on every
 * day it goes out, and the kind is the glyph at the end. A release that went
 * out is `done` - a weaker fill and quieter ink, still readable, where kilna
 * drew it at 60% until v0.110 and its words measured 2.7:1.
 *
 * The title takes what is left, the marks after it are a glyph each. The line
 * is a container, so the cover, the hour and the work's stage come back only
 * where it is wide enough for them to cost the title nothing - a day of the
 * week, a row of the agenda.
 *
 * The same line everywhere a release is drawn on the calendar: the month, the
 * week, the agenda, and the copy that travels under the pointer.
 */
export function ReleaseLine({ slot, date, now, dragging, onGrab, onOpen, carried = false }: Props) {
  const covers = useCovers()
  const { t } = useTranslation()
  const profile = useProfile()
  const releaseKinds = allOf(profile.config, 'release_kinds')

  const kind = releaseKinds.find((entry) => entry.key === slot.kind)
  const kindLabel = labelOf(releaseKinds, slot.kind)
  const released = slot.status === 'released'
  const gaps = missing(slot.readiness)
  const stage = stageAt(profile.config, slot.work_stage)
  // The guard's mark of the song this release is made from (ADR 0054): a
  // month is where a song is placed, so it is where a repeat is seen.
  const repeat = useRepeatMark(slot.work_id)

  // What the work is, beside what kind of release this is. Two kinds of work
  // may ship the same kind of release under the same glyph - a video's
  // YouTube release and a song's clip both say "film" - and then the glyph
  // alone cannot tell a video from a song. Said only while the profile has
  // more than one kind of work, and only in the tooltip and the card: a
  // one-line chip has no room for a word.
  const manyKinds = profile.config.work_kinds.length > 1
  const workKindLabel = labelOf(profile.config.work_kinds, slot.work_kind)
  const what = manyKinds ? `${workKindLabel} · ${kindLabel}` : kindLabel

  // The word for everyone the glyphs do not reach. The card below is a
  // shortcut for people who can see it hover; this is the fact itself.
  const label = `${slot.work_title} · ${what}`

  const line = (
    <BigCalendarItem
      color={accentFor(slot.work_id)}
      done={released}
      title={label}
      onPointerDown={released || carried ? undefined : onGrab}
      onClick={
        carried
          ? undefined
          : (event) => {
              // The day underneath books a date; the line opens what is
              // already booked. A press that turned into a drag never
              // reaches here: the gesture starts only after the pointer has
              // travelled.
              event.stopPropagation()
              onOpen()
            }
      }
      className={cn(
        '@container',
        // A released release still opens; only a planned one moves.
        !carried && !released && 'cursor-grab',
        // The line left behind while its copy travels: dimmed, so the day it
        // came from still reads as spoken for.
        dragging && 'opacity-30',
        // The copy is under the pointer; it must not answer
        // `elementFromPoint` instead of the day beneath it.
        carried && 'pointer-events-none',
      )}
      start={
        <>
          {/* The cover, where there is room for it: what the work looks like,
              beside the colour it has always had. */}
          <span
            aria-hidden
            className="hidden size-3.5 shrink-0 rounded-xs @min-[7rem]:block"
            style={{ background: coverImageFor(slot.work_id, covers.get(slot.work_id)) }}
          />
          {/* The hour, where the line is wide enough to say it before the
              title - a week's day on a wide window, the agenda's rows. */}
          {slot.scheduled_time !== null && (
            <span className="hidden font-mono text-2xs font-normal tabular-nums opacity-80 @min-[10rem]:inline">
              {slot.scheduled_time}
            </span>
          )}
        </>
      }
      end={
        <>
          {/* First among the marks: whether it could go out at all is about
              this release; whether it repeats a song is about where it is.
              No tooltip of its own - the card below says it in words. */}
          <RepeatMark mark={repeat} tooltip={false} />
          <ReadyMark release={slot} daysLeft={daysBetween(now, date)} />
          {slot.slot_pinned_at !== null && <Lock aria-hidden className="size-2.5" />}
          {slot.work_stage !== null && (
            <StageDial
              percent={slot.work_stage}
              stage={stage}
              size={10}
              className="hidden @min-[7rem]:block"
            />
          )}
          <KindGlyph icon={kind?.icon} className="size-3" />
        </>
      }
    >
      {slot.work_title}
    </BigCalendarItem>
  )

  // A line travelling under the pointer is only a picture of itself: no hover
  // card, which would open over the day it is being carried to.
  if (carried) return line

  return (
    <PreviewCard>
      <PreviewCardTrigger render={line} />

      {/* Everything here is also in the release dialog one click away. The card
          is not reachable by touch and not announced by a screen reader, so it
          may only ever be a shortcut - never the one place a fact lives. */}
      <PreviewCardPopup size="sm" className="p-3">
        <p className="text-sm font-semibold leading-tight">{slot.work_title}</p>
        <p className="mt-0.5 flex items-center gap-1.5 text-xs text-dim">
          <KindGlyph icon={kind?.icon} className="size-3.5 shrink-0" />
          {what}
        </p>

        <dl className="mt-2 flex flex-col gap-1 text-xs">
          <div className="flex justify-between gap-3">
            <dt className="text-faint">{t('calendar.previewScore')}</dt>
            <dd className="font-mono tabular-nums">
              {slot.total === null ? t('calendar.previewUnscored') : formatNumber(slot.total)}
            </dd>
          </div>
          {slot.tier !== null && (
            <div className="flex justify-between gap-3">
              <dt className="text-faint">{t('calendar.previewTier')}</dt>
              <dd>{labelOf(allOf(profile.config, 'tiers'), slot.tier)}</dd>
            </div>
          )}
          {/* The hour and whose clock it is on: the line's face has the hour
              only where it is wide. */}
          {slot.scheduled_time !== null && (
            <div className="flex justify-between gap-3">
              <dt className="text-faint">{t('calendar.slotTime')}</dt>
              <dd className="text-right">
                <span className="font-mono tabular-nums">{slot.scheduled_time}</span>
                {slot.time_zone !== null && (
                  <span className="text-dim"> · {zoneLabel(slot.time_zone)}</span>
                )}
              </dd>
            </div>
          )}
          {/* The stage, which a narrow line leaves off its face. */}
          <div className="flex justify-between gap-3">
            <dt className="text-faint">{t('stage.label')}</dt>
            <dd className="flex items-center gap-1.5">
              <StageDial percent={slot.work_stage} stage={stage} size={12} />
              {stage === undefined ? t('stage.unset') : sayLabel(stage.label)}
            </dd>
          </div>
          {repeat !== undefined && (
            <div className="flex justify-between gap-3">
              <dt className="text-faint">{t('repeats.caption')}</dt>
              <dd className={cn('text-right', repeat.level === 'red' ? 'text-bad' : 'text-warn')}>
                {markLabel(repeat)}
              </dd>
            </div>
          )}
          <div className="flex justify-between gap-3">
            <dt className="text-faint">{t('calendar.previewState')}</dt>
            <dd className={cn(released || gaps.length === 0 ? 'text-good' : 'text-warn')}>
              {released
                ? t('calendar.released')
                : gaps.length === 0
                  ? t('calendar.ready')
                  : t('calendar.notReadyHint', {
                      list: gaps.map((gap) => sayNeed(gap, profile.config, slot, t)).join(', '),
                    })}
            </dd>
          </div>
        </dl>

        {/* The link is the one thing here worth reaching for from the card
            itself. It goes through the opener rather than an anchor: inside a
            WebView `target="_blank"` reaches no browser at all. */}
        {slot.url !== null && slot.url !== '' && (
          <Button
            variant="link"
            onClick={() => void openExternal(slot.url ?? '')}
            title={slot.url}
            className="mt-2 max-w-full text-xs"
          >
            <span className="truncate">{shortLink(slot.url)}</span>
          </Button>
        )}
      </PreviewCardPopup>
    </PreviewCard>
  )
}

/**
 * Where the auto-layout, a shift or an imported plan would put a release: the
 * booked line, `tentative` - dashed, in the work's own colour as an outline -
 * so a plan is visibly not a booking.
 *
 * Inert on purpose: the plan is approved or cancelled from the bar above the
 * month, not edited line by line.
 */
export function PlannedLine({ ghost }: { ghost: Ghost }) {
  const profile = useProfile()
  const releaseKinds = allOf(profile.config, 'release_kinds')
  const kind = releaseKinds.find((entry) => entry.key === ghost.kind)

  return (
    <BigCalendarItem
      tentative
      color={accentFor(ghost.workId)}
      title={`${ghost.title} · ${labelOf(releaseKinds, ghost.kind)}`}
      end={<KindGlyph icon={kind?.icon} className="size-3 text-faint" />}
    >
      {ghost.title}
    </BigCalendarItem>
  )
}
