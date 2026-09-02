import { useTranslation } from 'react-i18next'
import { GripVertical, Lock } from 'lucide-react'
import type { ScheduledRelease } from '@/lib/api'
import { coverFor } from '@/lib/cover'
import { KindGlyph } from '@/lib/releaseIcon'
import { daysBetween, missing } from '@/lib/readiness'
import { openExternal, shortLink } from '@/lib/link'
import { labelOf, useProfile } from '@/lib/useProfile'
import { ReadyMarks } from '@/components/calendar/ReadyMarks'
import { PreviewCard, PreviewCardPopup, PreviewCardTrigger } from '@/components/ui/preview-card'
import { cn } from '@/lib/utils'

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
 * One booked release on a day.
 *
 * A column, not a row. A day is ~130px wide, and one row spent all of it on the
 * handle, the marks and the kind: the pilot's calendar showed three letters of
 * every title. The handle and the marks share the top line; the title gets the
 * day's full width underneath.
 */
export function SlotChip({ slot, date, now, dragging, onGrab, onOpen, asGhost = false }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()

  const kind = profile.config.release_kinds.find((entry) => entry.key === slot.kind)
  const kindLabel = labelOf(profile.config.release_kinds, slot.kind)
  const released = slot.status === 'released'
  const gaps = missing(slot.readiness)

  // The word for everyone the glyph does not reach. The card below is a
  // shortcut for people who can see it hover; this is the fact itself.
  const label = `${slot.work_title} · ${kindLabel}`

  const body = (
    <div
      onPointerDown={released || asGhost ? undefined : onGrab}
      className={cn(
        'flex flex-col gap-0.5 rounded-[7px] px-1 py-1 text-[11px]',
        !asGhost && 'transition-opacity',
        released && 'opacity-60',
        // The chip left behind while its copy travels: dimmed, so the day it
        // came from still reads as spoken for.
        dragging && 'opacity-30',
        !released && !asGhost && 'cursor-grab',
        // The ghost is under the pointer; it must not answer `elementFromPoint`
        // instead of the day beneath it, and it must not be grabbed again.
        asGhost && 'pointer-events-none scale-[1.03] shadow-float',
      )}
      // The work's own colour, so the same song is the same colour wherever
      // it appears — the cover, the card, this chip.
      style={{ background: coverFor(slot.work_id) }}
    >
        {/* The top line: what you grab and what the release's state is. Both
            are small and fixed-width, so they cost the title nothing. */}
        <div className="flex items-center gap-1">
          {/* The grip no longer owns the gesture — the whole chip does, and a
              press only becomes a drag after `DRAG_THRESHOLD`. It stays as the
              sign that the chip can be moved at all, which was the other half
              of its job. */}
          <span
            aria-hidden
            title={t('calendar.dragHandle')}
            className={cn('shrink-0 text-white/50', released && 'opacity-30')}
          >
            <GripVertical className="size-3" />
          </span>

          <button
            type="button"
            onClick={(event) => {
              // The day underneath books a date; the chip opens what is already
              // booked. One click means one of those.
              event.stopPropagation()
              onOpen()
            }}
            title={label}
            className="flex min-w-0 flex-1 cursor-pointer items-center justify-end gap-1.5 text-left hover:opacity-80"
          >
            <ReadyMarks
              readiness={slot.readiness}
              released={released}
              daysLeft={daysBetween(now, date)}
              // The chip's ground is the work's own colour; the dark pill keeps
              // the amber and red legible on any of them.
              className={cn('rounded-[4px] bg-black/35 px-0.5 py-px', released && 'text-white/70')}
            />
            {slot.slot_pinned_at !== null && (
              <Lock aria-hidden className="size-2.5 shrink-0 text-white/70" />
            )}
            {/* The kind, as the glyph its profile names. Two letters stood here
                while the code was not allowed to know which kinds exist
                (ADR 0001) — the profile now names the glyph too, so the rule
                holds and the mark is legible at a glance. The word stays in the
                tooltip, which is where a reader who cannot see the glyph finds
                it. */}
            <KindGlyph icon={kind?.icon} className="size-3 shrink-0 text-white/70" />
          </button>
        </div>

      {/* The title, on its own line. It opens the release; a press that turns
          into a drag never reaches the click, because the gesture starts only
          after the pointer has travelled. */}
      <button
        type="button"
        onClick={(event) => {
          event.stopPropagation()
          onOpen()
        }}
        title={label}
        className="w-full cursor-pointer truncate text-left font-medium leading-tight text-white/90 hover:opacity-80"
      >
        {slot.work_title}
      </button>
    </div>
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
          may only ever be a shortcut — never the one place a fact lives. */}
      <PreviewCardPopup size="sm" className="p-3">
        <p className="text-sm font-semibold leading-tight">{slot.work_title}</p>
        <p className="mt-0.5 flex items-center gap-1.5 text-xs text-dim">
          <KindGlyph icon={kind?.icon} className="size-3.5 shrink-0" />
          {kindLabel}
        </p>

        <dl className="mt-2 flex flex-col gap-1 text-xs">
          <div className="flex justify-between gap-3">
            <dt className="text-faint">{t('calendar.previewScore')}</dt>
            <dd className="font-mono tabular-nums">
              {slot.total === null ? t('calendar.previewUnscored') : slot.total.toFixed(1)}
            </dd>
          </div>
          {slot.tier !== null && (
            <div className="flex justify-between gap-3">
              <dt className="text-faint">{t('calendar.previewTier')}</dt>
              <dd>{labelOf(profile.config.tiers, slot.tier)}</dd>
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
                      list: gaps
                        .map((gap) =>
                          gap === 'score'
                            ? t('calendar.missingScore')
                            : labelOf(profile.config.version_roles, gap),
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
          <button
            type="button"
            onClick={() => void openExternal(slot.url ?? '')}
            title={slot.url}
            className="mt-2 block max-w-full truncate text-left text-xs text-accent-2 underline underline-offset-2"
          >
            {shortLink(slot.url)}
          </button>
        )}
      </PreviewCardPopup>
    </PreviewCard>
  )
}
