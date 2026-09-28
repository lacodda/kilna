import { useTranslation } from 'react-i18next'
import { Lock } from 'lucide-react'
import type { ScheduledRelease } from '@/lib/api/types'
import { accentFor, coverImageFor } from '@/lib/cover'
import { useCovers } from '@/lib/useCovers'
import { KindGlyph } from '@/lib/releaseIcon'
import { daysBetween, missing } from '@/lib/readiness'
import { openExternal, shortLink } from '@/lib/link'
import { allOf, labelOf, say as sayLabel, useProfile } from '@/lib/useProfile'
import { StageDial } from '@/components/StageDial'
import { stageAt } from '@/lib/stages'
import { Button } from '@/components/ui/button'
import { PreviewCard, PreviewCardPopup, PreviewCardTrigger } from '@/components/ui/preview-card'
import { ReadyMark } from '@/features/calendar/ReadyMark'
import { cn } from '@/lib/utils'
import { formatNumber } from '@/lib/format'

interface Props {
  slot: ScheduledRelease
  /** The day the chip sits on, for how urgent its gaps are. */
  date: string
  /** Today, so urgency is measured from one clock rather than each chip's own. */
  now: string
  dragging: boolean
  /** A press landed on the chip; the hook decides whether it becomes a drag. */
  onGrab: (event: React.PointerEvent) => void
  onOpen: () => void
  /** Drawn as the thing under the pointer rather than as a chip on a day. */
  asGhost?: boolean
}

/**
 * The colours of a chip: the work's own, as a soft fill with ink of the same
 * hue. The mockup fills a chip by the kind of release; the chip stays the
 * work's colour (decision 01.09, kept on 24.09), so the same song is the same
 * colour on every day it goes out - and the kind is the glyph at the end.
 *
 * Mixed with the theme rather than written down: the fill is the work's colour
 * over whatever the panel is, and the ink leans toward the theme's text, so it
 * is light on the dark theme and dark on the light one, and reads on both.
 */
function chipColours(workId: string) {
  const accent = accentFor(workId)
  return {
    background: `color-mix(in oklab, ${accent} 30%, transparent)`,
    color: `color-mix(in oklab, ${accent} 30%, var(--text))`,
  }
}

/**
 * One booked release on a day, in one line.
 *
 * Two lines stood here until v0.79 - the marks on top, the title under them -
 * and they were what made a busy week taller than the window. One line, the
 * mockup's: the title takes what is left, the marks after it are a glyph each.
 * The chip is a container, so its cover and the work's stage come back only
 * where it is wide enough for them to cost the title nothing.
 */
export function SlotChip({ slot, date, now, dragging, onGrab, onOpen, asGhost = false }: Props) {
  const covers = useCovers()
  const { t } = useTranslation()
  const profile = useProfile()
  const releaseKinds = allOf(profile.config, 'release_kinds')

  const kind = releaseKinds.find((entry) => entry.key === slot.kind)
  const kindLabel = labelOf(releaseKinds, slot.kind)
  const released = slot.status === 'released'
  const gaps = missing(slot.readiness)
  const stage = stageAt(profile.config, slot.work_stage)

  // What the work is, beside what kind of release this is. Two kinds of work
  // may ship the same kind of release under the same glyph - a video's
  // YouTube release and a song's clip both say "film" - and then the glyph
  // alone cannot tell a video from a song. Said only while the profile has
  // more than one kind of work: with one there is nothing to tell apart. On
  // the chip's face it was a word in ten-pixel capitals, and a one-line chip
  // has no room for it: it is said in the tooltip and in the card.
  const manyKinds = profile.config.work_kinds.length > 1
  const workKindLabel = labelOf(profile.config.work_kinds, slot.work_kind)
  const what = manyKinds ? `${workKindLabel} · ${kindLabel}` : kindLabel

  // The word for everyone the glyphs do not reach. The card below is a
  // shortcut for people who can see it hover; this is the fact itself.
  const label = `${slot.work_title} · ${what}`

  const body = (
    // eslint-disable-next-line dowel/no-raw-button -- drawn on the work's own colour, where every Button variant's ink and hover (tokens made for the theme's surfaces) would not read
    <button
      type="button"
      data-chip
      onPointerDown={released || asGhost ? undefined : onGrab}
      onClick={(event) => {
        // The day underneath books a date; the chip opens what is already
        // booked. One click means one of those. A press that turned into a
        // drag never reaches here: the gesture starts only after the pointer
        // has travelled.
        event.stopPropagation()
        onOpen()
      }}
      title={label}
      className={cn(
        '@container flex h-5 w-full min-w-0 shrink-0 items-center gap-1 rounded-sm px-1 text-left text-xs font-semibold',
        'focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent',
        !asGhost && 'transition-opacity hover:inset-ring hover:inset-ring-line-2',
        // A released chip still opens; only a planned one moves.
        !asGhost && (released ? 'cursor-pointer' : 'cursor-grab'),
        released && 'opacity-60',
        // The chip left behind while its copy travels: dimmed, so the day it
        // came from still reads as spoken for.
        dragging && 'opacity-30',
        // The ghost is under the pointer; it must not answer `elementFromPoint`
        // instead of the day beneath it, and it must not be grabbed again.
        asGhost && 'pointer-events-none',
      )}
      style={chipColours(slot.work_id)}
    >
      {/* The cover, where there is room for it: what the work looks like,
          beside the colour it has always had. */}
      <span
        aria-hidden
        className="hidden size-3.5 shrink-0 rounded-xs @min-[7rem]:block"
        style={{ background: coverImageFor(slot.work_id, covers.get(slot.work_id)) }}
      />
      <span className="min-w-0 flex-1 truncate">{slot.work_title}</span>

      {/* The marks, a glyph each and none of them a word, packed tighter
          than the title sits from them: whether it could go out, whether its
          date is kept, how far along the work is (where there is room), and
          the kind, last - the glyph its profile names. */}
      <span className="flex shrink-0 items-center gap-0.5">
        <ReadyMark
          readiness={slot.readiness}
          released={released}
          daysLeft={daysBetween(now, date)}
        />
        {slot.slot_pinned_at !== null && <Lock aria-hidden className="size-2.5 shrink-0" />}
        {slot.work_stage !== null && (
          <StageDial
            percent={slot.work_stage}
            stage={stage}
            size={10}
            className="hidden @min-[7rem]:block"
          />
        )}
        <KindGlyph icon={kind?.icon} className="size-3 shrink-0" />
      </span>
    </button>
  )

  // A chip travelling under the pointer is only a picture of itself: no hover
  // card, which would open over the day it is being carried to, and nothing
  // else to interact with.
  if (asGhost) return body

  return (
    <PreviewCard>
      <PreviewCardTrigger render={body} />

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
          {/* The stage, which a narrow chip leaves off its face. */}
          <div className="flex justify-between gap-3">
            <dt className="text-faint">{t('stage.label')}</dt>
            <dd className="flex items-center gap-1.5">
              <StageDial percent={slot.work_stage} stage={stage} size={12} />
              {stage === undefined ? t('stage.unset') : sayLabel(stage.label)}
            </dd>
          </div>
          <div className="flex justify-between gap-3">
            <dt className="text-faint">{t('calendar.previewState')}</dt>
            <dd className={cn(released || gaps.length === 0 ? 'text-good' : 'text-warn')}>
              {released
                ? t('calendar.released')
                : gaps.length === 0
                  ? t('calendar.ready')
                  : t('calendar.notReadyHint', {
                      list: gaps
                        .map((gap) =>
                          gap === 'score'
                            ? t('calendar.missingScore')
                            : labelOf(allOf(profile.config, 'version_roles'), gap),
                        )
                        .join(', '),
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
