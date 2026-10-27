import { useTranslation } from 'react-i18next'
import type { TFunction } from 'i18next'
import { Plus } from 'lucide-react'
import type { DonorParts, OfferedPart } from '@/lib/api/types'
import { shareInside, type Stretch } from '@/lib/cuts'
import { formatSeconds } from '@/lib/timecode'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'

/** The words for each reason a part is offered; a key a newer plugin knows
    and this window does not is shown as it is. */
const REASONS: Record<string, string> = {
  chorus: 'cuts.parts.reason.chorus',
  loudest: 'cuts.parts.reason.loudest',
  repeats: 'cuts.parts.reason.repeats',
  lift: 'cuts.parts.reason.lift',
  part: 'cuts.parts.reason.part',
  wordsChorus: 'cuts.parts.reason.wordsChorus',
  wordsHook: 'cuts.parts.reason.wordsHook',
}

/** How much of a part must lie in a sibling's stretch for the part to be
    said to be taken already. */
const TAKEN_SHARE = 0.5

interface LaneProps {
  parts: DonorParts
  /** The donor's length, to draw the lane against. */
  duration: number
}

/**
 * Where a short could be cut from one video (v0.103), drawn under its track:
 * a part offered as a dashed outline - the accent for what a plugin heard,
 * green for what the words place - and a stretch a short's sibling took as a
 * grey band, each with its reason on hover. Read by the eye; the list says
 * the same in words and takes a part.
 */
export function PartsLane({ parts, duration }: LaneProps) {
  const { t } = useTranslation()
  if (parts.parts.length === 0 && parts.taken.length === 0) return null
  return (
    <div
      data-lane=""
      aria-hidden
      className="relative h-3 rounded-sm border border-line bg-bg"
      title={t('cuts.parts.lane')}
    >
      {parts.taken.map((stretch) => (
        <span
          key={`taken-${stretch.work_id}-${stretch.starts_at}`}
          title={`${t('cuts.parts.takenBy', { title: stretch.title })} · ${spanOf(stretch)}`}
          style={place(stretch, duration)}
          className="absolute inset-y-0 bg-line-2"
        />
      ))}
      {parts.parts.map((part) => (
        <span
          key={`${part.source}-${part.starts_at}`}
          title={`${reasonOf(t, part)} · ${spanOf(part)}`}
          style={place(part, duration)}
          className={cn(
            'absolute inset-y-0 rounded-sm border border-dashed',
            part.source === 'sound' ? 'border-accent bg-accent/10' : 'border-good bg-good/10',
          )}
        />
      ))}
    </div>
  )
}

interface ListProps {
  title: string
  parts: DonorParts
  onTake: (part: Stretch) => void
  busy: boolean
}

/**
 * The parts offered for one video in words, each with **Take**, which makes
 * it a stretch like any other - Ctrl+Z takes it back - and a word on what
 * was not heard: a video nobody listened to, or one rendered again since.
 * A part that lies mostly in a stretch a sibling took says so. Nothing is a
 * stretch until it is taken; the person picks.
 */
export function PartsList({ title, parts, onTake, busy }: ListProps) {
  const { t } = useTranslation()
  const takenBy = (part: OfferedPart) =>
    parts.taken.find((stretch) => shareInside(part, stretch) >= TAKEN_SHARE)

  const said =
    parts.hearing === 'unheard'
      ? t('cuts.parts.unheard')
      : parts.hearing === 'stale'
        ? t('cuts.parts.stale')
        : parts.hearing === 'heard' && !parts.parts.some((part) => part.source === 'sound')
          ? t('cuts.parts.nothingHeard')
          : null
  if (parts.parts.length === 0 && said === null) return null

  return (
    <section aria-label={t('cuts.parts.title', { title })} className="flex flex-col gap-1">
      {said !== null && <p className="text-xs text-dim">{said}</p>}
      {parts.parts.length > 0 && (
        <ul className="flex flex-col gap-0.5">
          {parts.parts.map((part) => {
            const sibling = takenBy(part)
            return (
              <li
                key={`${part.source}-${part.starts_at}`}
                className="flex min-w-0 items-center gap-2 text-sm"
              >
                <span className="w-28 shrink-0 font-mono text-xs whitespace-nowrap text-dim">
                  {spanOf(part)}
                </span>
                <span className="w-10 shrink-0 font-mono text-xs text-faint">
                  {formatSeconds(part.ends_at - part.starts_at)}
                </span>
                <span className="min-w-0 truncate" title={part.detail ?? undefined}>
                  {reasonOf(t, part)}
                  {part.detail !== null && <span className="text-faint"> · {part.detail}</span>}
                </span>
                {sibling !== undefined && (
                  <span className="shrink-0 truncate text-xs text-faint">
                    {t('cuts.parts.alreadyIn', { title: sibling.title })}
                  </span>
                )}
                <Button
                  size="sm"
                  variant="ghost"
                  className="ml-auto shrink-0"
                  disabled={busy}
                  aria-label={t('cuts.parts.takeThis', { span: spanOf(part) })}
                  onClick={() => onTake({ starts_at: part.starts_at, ends_at: part.ends_at })}
                >
                  <Plus aria-hidden />
                  {t('cuts.parts.take')}
                </Button>
              </li>
            )
          })}
        </ul>
      )}
    </section>
  )
}

function reasonOf(t: TFunction, part: OfferedPart): string {
  const key = REASONS[part.reason]
  return key === undefined ? part.reason : t(key)
}

function spanOf(stretch: Stretch): string {
  return `${formatSeconds(stretch.starts_at)} – ${formatSeconds(stretch.ends_at)}`
}

/** Where a stretch sits on a lane of `duration` seconds, as CSS. */
function place(stretch: Stretch, duration: number) {
  const left = Math.max(0, Math.min(100, (stretch.starts_at / duration) * 100))
  const right = Math.max(0, Math.min(100, (stretch.ends_at / duration) * 100))
  return { left: `${left}%`, width: `${Math.max(0.5, right - left)}%` }
}
