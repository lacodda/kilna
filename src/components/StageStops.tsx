import { useTranslation } from 'react-i18next'
import type { Stage } from '@/lib/api/types'
import { stagesOf } from '@/lib/stages'
import { useProfile } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { StageDial, stageWords } from '@/components/StageDial'

interface Props {
  /** What the row is, for a screen reader. */
  label: string
  /** The stops that are on: the one a work stands at, or the ones ticked. */
  on: readonly number[]
  /**
   * `one` is a choice of one stop - a work's stage: the row is a radio group,
   * and the arrow keys move along it. `any` is a set of ticks - a filter:
   * each dial is a toggle.
   */
  pick: 'one' | 'any'
  onPick: (percent: number) => void
  disabled?: boolean
  className?: string
}

/**
 * The five stops in a row, as dials to press (FR-03).
 *
 * The one control a stage is set and filtered by: the picker in a card's
 * header and a catalogue's row, the widget on the overview, the funnel of the
 * stage column and the calendar's queue. No word and no number stands beside
 * a dial - five words in a row are a paragraph, and the dial already draws
 * its fraction; what a stop is called is said when it is pointed at, and read
 * to a screen reader as its name.
 */
export function StageStops({ label, on, pick, onPick, disabled = false, className }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const stops = stagesOf(profile.config)
  const one = pick === 'one'
  const current = stops.findIndex((stop) => on.includes(stop.percent))

  const move = (delta: number) => {
    const next = stops[Math.min(Math.max(current + delta, 0), stops.length - 1)]
    if (next !== undefined && next.percent !== stops[current]?.percent) onPick(next.percent)
  }

  return (
    <div
      role={one ? 'radiogroup' : 'group'}
      aria-label={label}
      onKeyDown={
        one
          ? (event) => {
              switch (event.key) {
                case 'ArrowRight':
                case 'ArrowUp':
                  event.preventDefault()
                  move(1)
                  break
                case 'ArrowLeft':
                case 'ArrowDown':
                  event.preventDefault()
                  move(-1)
                  break
                case 'Home':
                  event.preventDefault()
                  onPick(stops[0]!.percent)
                  break
                case 'End':
                  event.preventDefault()
                  onPick(stops[stops.length - 1]!.percent)
                  break
                default:
                  break
              }
            }
          : undefined
      }
      className={cn('flex items-center gap-1', className)}
    >
      {stops.map((stop: Stage, at) => {
        const isOn = on.includes(stop.percent)
        const words = stageWords(t, stop)
        return (
          <Button
            key={stop.percent}
            variant="icon"
            size="icon-sm"
            role={one ? 'radio' : undefined}
            aria-checked={one ? isOn : undefined}
            aria-pressed={one ? undefined : isOn}
            aria-label={words}
            title={words}
            disabled={disabled}
            // A radio group is one tab stop, on its chosen stop.
            tabIndex={one && at !== Math.max(current, 0) ? -1 : 0}
            onClick={() => onPick(stop.percent)}
            // The stop that is on stands on a plate; the dial itself is
            // drawn the same either way, so the five still read as a scale.
            className={cn(isOn && 'bg-soft ring-1 ring-line-2 ring-inset')}
          >
            <StageDial stage={stop} size={16} />
          </Button>
        )
      })}
    </div>
  )
}
