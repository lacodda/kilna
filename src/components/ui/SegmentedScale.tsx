import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'

interface Props {
  /** Highest mark on this axis; the control draws one segment per whole point. */
  scale: number
  /** Current mark, or `undefined` while the axis is unjudged. */
  value: number | undefined
  onChange: (value: number | undefined) => void
  label: string
  /**
   * The mark from which the total would cross into the next tier, and what to
   * call it. Absent when no mark on this axis gets there, and then nothing is
   * drawn: a line promising a tier the axis cannot deliver is worse than no
   * line at all.
   */
  threshold?: { mark: number; label: string }
  /** What the mark under the pointer means, if the profile names it. */
  onPreview?: (mark: number | undefined) => void
  className?: string
}

/**
 * A row of segments instead of a number field.
 *
 * Scoring is a judgement, not data entry: the useful question is "is this a
 * seven or an eight", and a row you click answers it in one movement where a
 * spin box asks you to read, aim and type. The filled length is also readable
 * at a glance across a column of axes, which a column of numerals is not.
 *
 * It reports itself as a slider rather than a group of buttons, so the arrow
 * keys, Home and End work the way they do everywhere else — and so a screen
 * reader announces a value out of a range instead of ten unlabelled buttons.
 */
export function SegmentedScale({
  scale,
  value,
  onChange,
  label,
  threshold,
  onPreview,
  className,
}: Props) {
  const { t } = useTranslation()
  const marks = Math.max(1, Math.round(scale))

  const step = (delta: number) => {
    // An unjudged axis starts from the first mark rather than from zero: zero is
    // a verdict of its own, and arrowing into it by accident would be one.
    const next = value === undefined ? (delta > 0 ? 1 : marks) : value + delta
    onChange(Math.min(Math.max(next, 0), marks))
  }

  const onKeyDown = (event: React.KeyboardEvent) => {
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
        onChange(0)
        break
      case 'End':
        event.preventDefault()
        onChange(marks)
        break
      // Backspace and Delete take the axis back to unjudged, which is not the
      // same as scoring it zero — see `lib/scoring`.
      case 'Backspace':
      case 'Delete':
        event.preventDefault()
        onChange(undefined)
        break
      default:
        break
    }
  }

  return (
    <div
      role="slider"
      tabIndex={0}
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={marks}
      aria-valuenow={value}
      aria-valuetext={value === undefined ? t('score.unjudged') : String(value)}
      onKeyDown={onKeyDown}
      onMouseLeave={() => onPreview?.(undefined)}
      className={cn(
        'flex gap-[3px] rounded-[7px] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent',
        className,
      )}
    >
      {Array.from({ length: marks }, (_, index) => {
        const mark = index + 1
        const filled = value !== undefined && mark <= value

        // The first mark that would carry the total over: the line goes on its
        // leading edge, so "from here" reads as the boundary it is.
        const crosses = threshold !== undefined && threshold.mark === mark

        return (
          <button
            key={mark}
            type="button"
            // The row owns the keyboard; the segments are pointer targets only,
            // or tabbing through one axis would take ten presses.
            tabIndex={-1}
            aria-hidden
            // Clicking the mark you are already on clears the axis, which is
            // the only way back to unjudged with the mouse.
            onClick={() => onChange(value === mark ? undefined : mark)}
            onMouseEnter={() => onPreview?.(mark)}
            title={crosses ? threshold.label : undefined}
            className={cn(
              'relative h-[22px] flex-1 cursor-pointer rounded-[5px] transition-colors',
              filled ? 'bg-accent hover:bg-accent-2' : 'bg-soft hover:bg-line-2',
              // An outline around the mark itself rather than a rule beside it.
              // A line on the leading edge reads as the end of the scale when
              // the mark is the last one - which is exactly when the threshold
              // matters most. Ringing the segment says "this one" wherever it
              // falls, and leaves the fill free to keep meaning "scored".
              crosses && 'ring-2 ring-inset ring-good',
            )}
          />
        )
      })}
    </div>
  )
}
