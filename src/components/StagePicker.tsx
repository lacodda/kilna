import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Popover, PopoverPopup, PopoverTrigger } from '@/components/ui/popover'
import { StageDial } from '@/components/StageDial'
import { stageAt, stagesOf } from '@/lib/stages'
import { say, useProfile } from '@/lib/useProfile'
import { useStage } from '@/lib/useStage'
import { cn } from '@/lib/utils'

/**
 * The dial, and the scale that sets it.
 *
 * A row of stops rather than a free slider: the stops are what the craft
 * actually thinks in — an idea, a rough take, polishing — and a continuous
 * slider would invite the author to agonise over 63 versus 67, which is a
 * question about the control rather than about the work.
 *
 * It reports itself as a slider all the same, so the arrow keys, Home and End
 * behave the way they do everywhere else and a screen reader announces a
 * position rather than six unlabelled buttons — the pattern `SegmentedScale`
 * set for the score axes.
 */
export function StagePicker({
  workId,
  percent,
  /** The button's size; the catalogue row draws it tighter than the card. */
  compact = false,
  className,
}: {
  workId: string
  percent: number | null
  compact?: boolean
  className?: string
}) {
  const { t } = useTranslation()
  const profile = useProfile()
  const [open, setOpen] = useState(false)
  const stage = useStage(workId)

  const stops = stagesOf(profile.config)
  const current = stageAt(profile.config, percent)
  const index = current === undefined ? -1 : stops.findIndex((stop) => stop.key === current.key)

  const set = (next: number | null) => {
    stage.mutate(next)
    // Choosing is the whole errand; the panel closes rather than waiting for a
    // second, dismissing click.
    setOpen(false)
  }

  const step = (delta: number) => {
    // Unjudged starts at the first stop rather than at the last: arrowing right
    // from "nothing said" means "begin", not "jump to finished".
    const target = index === -1 ? (delta > 0 ? 0 : stops.length - 1) : index + delta
    const clamped = Math.min(Math.max(target, 0), stops.length - 1)
    set(stops[clamped]?.percent ?? 0)
  }

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger
        aria-label={
          current === undefined
            ? t('stage.unset')
            : t('stage.atPercent', { percent: percent ?? 0, stage: say(current.label) })
        }
        title={current === undefined ? t('stage.set') : say(current.label)}
        disabled={stage.isPending}
        className={cn(
          'cursor-pointer rounded-md transition-colors hover:bg-soft',
          compact ? 'p-0.5' : 'flex size-7 items-center justify-center',
          className,
        )}
      >
        <StageDial percent={percent} stage={current} size={compact ? 14 : 16} />
      </PopoverTrigger>

      <PopoverPopup arrow={false} className="w-auto p-2">
        <div
          role="slider"
          tabIndex={0}
          aria-label={t('stage.label')}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={percent ?? undefined}
          aria-valuetext={current === undefined ? t('stage.unset') : say(current.label)}
          onKeyDown={(event) => {
            switch (event.key) {
              case 'ArrowRight':
              case 'ArrowUp':
                event.preventDefault()
                step(1)
                break
              case 'ArrowLeft':
              case 'ArrowDown':
                event.preventDefault()
                step(-1)
                break
              case 'Home':
                event.preventDefault()
                set(stops[0]?.percent ?? 0)
                break
              case 'End':
                event.preventDefault()
                set(stops[stops.length - 1]?.percent ?? 100)
                break
              // Back to "nobody has said", which no stop on the scale can
              // express — the only way out of having answered.
              case 'Backspace':
              case 'Delete':
                event.preventDefault()
                set(null)
                break
              default:
                break
            }
          }}
          className="flex items-center gap-1 rounded-md focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
        >
          {stops.map((stop, at) => {
            const reached = index >= at
            return (
              <button
                key={stop.key}
                type="button"
                // The row owns the keyboard; the stops are pointer targets, or
                // tabbing past the dial would take six presses.
                tabIndex={-1}
                title={`${say(stop.label)} · ${stop.percent}%`}
                aria-label={say(stop.label)}
                onClick={() => set(percent === stop.percent ? null : stop.percent)}
                className="group flex cursor-pointer flex-col items-center gap-1 rounded-sm px-1 py-0.5 transition-colors hover:bg-soft"
              >
                <StageDial percent={stop.percent} stage={stop} size={16} />
                {/* The number, not the word: six words in a row would be a
                    paragraph, and the word is a hover away. */}
                <span
                  className={cn(
                    'text-[10px] tabular-nums transition-colors',
                    reached ? 'font-semibold text-text' : 'text-faint',
                  )}
                >
                  {stop.percent}
                </span>
              </button>
            )
          })}

          {percent !== null && (
            <button
              type="button"
              tabIndex={-1}
              onClick={() => set(null)}
              className="ml-1 cursor-pointer self-stretch rounded-sm px-1.5 text-[10px] text-faint transition-colors hover:bg-soft hover:text-text"
            >
              {t('stage.clear')}
            </button>
          )}
        </div>
      </PopoverPopup>
    </Popover>
  )
}
