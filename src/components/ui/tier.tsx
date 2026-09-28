import { useId, type HTMLAttributes, type KeyboardEvent, type ReactNode } from 'react'
import { cva, type VariantProps } from 'class-variance-authority'
import { cn } from 'dowel-ui'

/*
 * Tiers: the band a number falls in, the road of bands, and one axis of the
 * score that produced it.
 *
 * The shape comes from kilna, where a work is scored on several axes and the
 * total lands in a named band - "draft", "publishable", "a clip". Two more
 * products of the line grade things the same way (a health score, a review
 * verdict), which is what makes this a primitive rather than a screen.
 *
 * The three pieces answer three different questions and are separate for that
 * reason:
 *
 * - `TierBadge` - *which band is this in?* A verdict, in one word.
 * - `TierRuler` - *how far into it, and how far to the next?* The verdict
 *   stops being a pronouncement out of nowhere: "nearly a clip" is the thing
 *   someone wants to know while they are still moving marks around.
 * - `AxisBar` - *what did one axis contribute?* A row of segments read left to
 *   right, which a column of numerals is not.
 *
 * The defect corrected on the way in, and the reason not to copy the donor as
 * it stands: its ruler drew the bands to scale and then laid the labels out
 * with `justify-between`, which spaces them evenly. With bands at 0, 50, 78
 * and 90 the label "78" sat a fifth of the bar away from the boundary it
 * named. Here the labels are placed by the same arithmetic as the bands, so
 * the distance you read is the distance there is.
 */

/** A band: where it starts, and what it is called.
 *
 * `min` is on the same scale as the value - 0-100 in every consumer so far,
 * but the components do not require it: `TierRuler` takes the ends explicitly.
 * The band runs from its `min` to the next band's, and the last one runs to
 * the end. */
export interface Tier {
  /** Stable key, for React and for the caller's own lookups. */
  key: string
  /** What it is called, in the product's words. */
  label: string
  /** The lowest value in this band. */
  min: number
  /** How the band is coloured. `accent` is the default - a band is a position
   * on the product's own scale, not a judgement in the status vocabulary. A
   * product that means "this one is bad" says so. */
  status?: 'accent' | 'good' | 'warn' | 'bad' | 'info' | 'neutral'
}

/**
 * The bands in order, with the ones that cannot be drawn dropped.
 *
 * Exported because a product that has tiers usually needs them sorted
 * somewhere else too, and because sorting inside a render is the kind of thing
 * that quietly becomes three different sorts.
 */
export function orderedTiers(tiers: Tier[], min = 0, max = 100): Tier[] {
  return tiers
    .filter((tier) => Number.isFinite(tier.min) && tier.min >= min && tier.min <= max)
    .sort((left, right) => left.min - right.min)
}

/**
 * Which band a value is standing in.
 *
 * The highest band whose `min` the value has reached. Below every band - which
 * a caller can reach by scoring under the first `min` - the answer is
 * `undefined`, and that is a real state rather than a reason to clamp: a
 * product that shows a badge for a band nothing is in would be inventing one.
 */
export function tierAt(tiers: Tier[], value: number): Tier | undefined {
  const ordered = orderedTiers(tiers)
  let standing: Tier | undefined
  for (const tier of ordered) {
    if (value >= tier.min) standing = tier
    else break
  }
  return standing
}

const tierFill: Record<NonNullable<Tier['status']>, string> = {
  accent: 'bg-accent',
  good: 'bg-good',
  warn: 'bg-warn',
  bad: 'bg-bad',
  info: 'bg-info',
  neutral: 'bg-line-2',
}

/** The same bands at a quarter strength, for the stretch already passed.
 *
 * Written out rather than composed with an opacity utility, because a fill and
 * an opacity are two properties and a caller's `className` can only win
 * against one of them. */
const tierPassedFill: Record<NonNullable<Tier['status']>, string> = {
  accent: 'bg-accent/40',
  good: 'bg-good/40',
  warn: 'bg-warn/40',
  bad: 'bg-bad/40',
  info: 'bg-info/40',
  neutral: 'bg-line-2/40',
}

export const tierBadgeVariants = cva(
  'inline-flex items-center gap-1.5 rounded-md px-2 py-0.5 text-xs font-medium whitespace-nowrap',
  {
    variants: {
      status: {
        accent: 'bg-accent-soft text-accent',
        good: 'bg-good-soft text-good',
        warn: 'bg-warn-soft text-warn',
        bad: 'bg-bad-soft text-bad',
        info: 'bg-info-soft text-info',
        neutral: 'bg-soft text-dim',
      },
    },
    defaultVariants: { status: 'accent' },
  },
)

export interface TierBadgeProps
  extends Omit<HTMLAttributes<HTMLSpanElement>, 'children'>,
    VariantProps<typeof tierBadgeVariants> {
  /** The band's name. */
  label: ReactNode
  /** The number behind the verdict, if the product shows it. Drawn quieter
   * than the name: the band is the answer and the score is the evidence. */
  value?: ReactNode
}

/**
 * The band, as a verdict.
 *
 * Square-cornered rather than a pill, which is what separates it from
 * `StatusBadge` at a glance: a tier is a position on a scale the product owns,
 * a status is a condition from the line's fixed vocabulary, and a screen that
 * shows both should not make the reader compare colours to tell which is
 * which.
 */
export function TierBadge({ label, value, status, className, ...props }: TierBadgeProps) {
  return (
    <span className={cn(tierBadgeVariants({ status }), className)} {...props}>
      {label}
      {value !== undefined && <span className="font-normal opacity-70">{value}</span>}
    </span>
  )
}

export interface TierRulerProps extends Omit<HTMLAttributes<HTMLDivElement>, 'children'> {
  tiers: Tier[]
  /** Where the thing stands now. */
  value: number
  /** The ends of the road. */
  min?: number
  max?: number
  /** What the ruler as a whole is called, for a screen reader, and what to say
   * the value is. The product's words - the ruler has none. */
  label: string
  /** The value spoken instead of the bare number: "7.8 out of 10, publishable"
   * is what a reader needs, and only the product can phrase it. */
  valueText?: string
  /** Print the band names under the road. */
  showLabels?: boolean
}

/**
 * The bands as a road, with the value standing somewhere along it.
 *
 * Drawn to scale, which is the whole point: a band that starts at 78 sits
 * nearly four fifths along, and the gap you are looking at is the gap you have
 * to close.
 *
 * Three states per band, not two - passed, standing in, still ahead. A flat
 * wash of "reached" over half the bar says only that the value is not at zero;
 * picking out the band being stood in is what carries the eye to where the
 * thing actually is.
 *
 * Reported as a `meter`: it is a measurement inside a known range, which is
 * exactly what that role is for, and it means a screen reader says the value
 * without the product building a sentence out of `aria-label`.
 */
export function TierRuler({
  tiers,
  value,
  min = 0,
  max = 100,
  label,
  valueText,
  showLabels = true,
  className,
  ...props
}: TierRulerProps) {
  const ordered = orderedTiers(tiers, min, max)
  const span = max - min

  // A road needs at least a start and one boundary on it; below that there is
  // nothing to show that a bare number would not say better.
  if (ordered.length < 2 || span <= 0) return null

  const at = Math.min(Math.max(value, min), max)
  /** Where a value falls along the road, as a percentage of its length. */
  const placeOf = (point: number) => ((point - min) / span) * 100

  return (
    <div className={cn('flex flex-col gap-1', className)} {...props}>
      <div
        role="meter"
        aria-label={label}
        aria-valuemin={min}
        aria-valuemax={max}
        aria-valuenow={at}
        aria-valuetext={valueText}
        className="relative h-1.5 w-full overflow-hidden rounded-full bg-soft"
      >
        {ordered.map((tier, index) => {
          const next = ordered[index + 1]
          const end = next === undefined ? max : next.min
          const width = placeOf(end) - placeOf(tier.min)
          if (width <= 0) return null

          const reached = value >= tier.min
          const standing = reached && (next === undefined || value < next.min)
          const status = tier.status ?? 'accent'

          return (
            <span
              key={tier.key}
              style={{ left: `${placeOf(tier.min)}%`, width: `${width}%` }}
              className={cn(
                'absolute top-0 h-full',
                standing ? tierFill[status] : reached ? tierPassedFill[status] : 'bg-line-2',
                // A hairline of the page's own ground, so two adjacent bands of
                // the same colour still read as two.
                index > 0 && 'border-l border-bg',
              )}
            />
          )
        })}

        {/* Where the value stands. A pale core inside a dark sheath, so the
            mark keeps its contrast over a filled band as well as over the
            empty road ahead. */}
        <span
          aria-hidden
          style={{ left: `${placeOf(at)}%` }}
          className="absolute top-1/2 h-2.5 w-1.5 -translate-x-1/2 -translate-y-1/2 rounded-full bg-bg ring-2 ring-text"
        />
      </div>

      {showLabels && (
        <div aria-hidden className="relative h-4">
          {ordered.map((tier, index) => {
            const place = placeOf(tier.min)
            return (
              <span
                key={tier.key}
                style={{
                  left: `${place}%`,
                  // The first label would hang off the left edge and the last
                  // off the right, so the ends align to their edge and
                  // everything between is centred on its boundary.
                  transform:
                    index === 0
                      ? 'none'
                      : index === ordered.length - 1 && place >= 100
                        ? 'translateX(-100%)'
                        : 'translateX(-50%)',
                }}
                className={cn(
                  'absolute top-0 text-2xs',
                  value >= tier.min ? 'font-medium text-dim' : 'text-faint',
                )}
              >
                {tier.label}
              </span>
            )
          })}
        </div>
      )}
    </div>
  )
}

export interface AxisBarProps extends Omit<HTMLAttributes<HTMLDivElement>, 'onChange'> {
  /** What this axis is called. */
  label: string
  /** The highest mark; one segment is drawn per whole point. */
  scale: number
  /** The mark, or `undefined` while the axis is unjudged. Not the same as
   * zero: zero is a verdict, and a blank axis is the absence of one. */
  value?: number
  /** Given, the row becomes a control. Omitted, it is a read-only reading of a
   * score someone else set. */
  onChange?: (value: number | undefined) => void
  /** The mark from which the total crosses into the next band, and what to
   * call it. Absent when no mark on this axis gets there - and then nothing is
   * drawn, because a line promising a band the axis cannot deliver is worse
   * than no line. */
  threshold?: { mark: number; label: string }
  /** What to say instead of the bare number - "unjudged", in the product's
   * word, when there is no mark. */
  valueText?: string
  /** The mark under the pointer, and `undefined` when the pointer leaves -
   * so a product can say what a mark means while it is being weighed, before
   * anyone commits to it. The question while scoring is about the mark being
   * considered, not the one already given. */
  onPreview?: (mark: number | undefined) => void
}

/**
 * One axis of a score, as a row of segments.
 *
 * Scoring is a judgement, not data entry: the useful question is "is this a
 * seven or an eight", and a row you click answers it in one movement where a
 * spin box asks you to read, aim and type. The filled length is also readable
 * down a column of axes, which numerals are not.
 *
 * When it takes `onChange` it reports itself as a slider rather than as a row
 * of buttons, so the arrow keys, Home and End work the way they do everywhere
 * else, and a reader hears one value in a range instead of ten unlabelled
 * buttons. The row owns the keyboard; the segments are pointer targets only,
 * or tabbing past one axis would take ten presses.
 *
 * Read-only, it is a `meter` and not a disabled slider: nothing here is
 * disabled, the number is simply a fact.
 */
export function AxisBar({
  label,
  scale,
  value,
  onChange,
  threshold,
  valueText,
  onPreview,
  className,
  ...props
}: AxisBarProps) {
  const marks = Math.max(1, Math.round(scale))
  const interactive = onChange !== undefined
  const thresholdId = useId()
  // Drawn only where it is true, and said only where it is drawn: a mark
  // past the end of the scale is no threshold at all.
  const reachable = threshold !== undefined && threshold.mark >= 1 && threshold.mark <= marks

  const step = (delta: number) => {
    if (!onChange) return
    // An unjudged axis steps onto the first or last mark rather than through
    // zero: zero is a verdict of its own, and arrowing into it by accident
    // would be one.
    const next = value === undefined ? (delta > 0 ? 1 : marks) : value + delta
    onChange(Math.min(Math.max(next, 0), marks))
  }

  const onKeyDown = (event: KeyboardEvent) => {
    if (!onChange) return
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
      // Back to unjudged, which no arrow key can reach - and which is not the
      // same as scoring the axis zero.
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
      role={interactive ? 'slider' : 'meter'}
      tabIndex={interactive ? 0 : undefined}
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={marks}
      aria-valuenow={value}
      aria-valuetext={valueText}
      // The threshold is a ring on a segment and a `title` on it, and both
      // are pointer-only: the segments are `aria-hidden`, and a title on a
      // span is never shown to the keyboard. So the row itself carries it as
      // its description, and a reader hears "a clip from 8" with the value.
      aria-describedby={reachable ? thresholdId : undefined}
      onKeyDown={interactive ? onKeyDown : undefined}
      onPointerLeave={onPreview ? () => onPreview(undefined) : undefined}
      className={cn(
        'flex gap-hair rounded-md focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent',
        className,
      )}
      {...props}
    >
      {Array.from({ length: marks }, (_, index) => {
        const mark = index + 1
        const filled = value !== undefined && mark <= value
        // The first mark that would carry the total over. The ring goes around
        // the segment rather than on its leading edge: a rule beside the last
        // mark reads as the end of the scale, which is exactly when the
        // threshold matters most.
        const crosses = reachable && threshold.mark === mark
        const preview = onPreview ? () => onPreview(mark) : undefined

        const shared = cn(
          'relative h-5 flex-1 rounded-sm transition-colors',
          filled ? 'bg-accent' : 'bg-soft',
          crosses && 'ring-2 ring-inset ring-good',
        )

        return interactive ? (
          // A `<span>` and not a `<button>`, which is what the donor used and
          // what the accessibility gate rejected: a focusable control inside a
          // `role="slider"` is a nested interactive element, and `tabindex=-1`
          // with `aria-hidden` does not undo that - assistive technology can
          // still land on it, and WCAG 4.1.2 says so.
          //
          // Nothing is lost. The row already owns the keyboard, so the segment
          // was never a keyboard target; it is a pointer target, which a span
          // with a click handler is.
          <span
            key={mark}
            aria-hidden
            // Clicking the mark already set clears the axis, which is the only
            // way back to unjudged with a pointer.
            onClick={() => onChange?.(value === mark ? undefined : mark)}
            onPointerEnter={preview}
            title={crosses ? threshold.label : undefined}
            className={cn(shared, 'cursor-pointer', filled ? 'hover:bg-accent-2' : 'hover:bg-line-2')}
          />
        ) : (
          <span key={mark} aria-hidden onPointerEnter={preview} className={shared} />
        )
      })}
      {reachable ? (
        <span id={thresholdId} hidden>
          {threshold.label}
        </span>
      ) : null}
    </div>
  )
}
