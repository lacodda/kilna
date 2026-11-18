import type { ReactNode } from 'react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'

/** One stop of a scale. */
export interface ScaleStep<Value extends string> {
  value: Value
  label: string
  /** A glyph drawn above the label, when the step has one. */
  icon?: ReactNode
}

interface Props<Value extends string> {
  label: string
  steps: readonly ScaleStep<Value>[]
  value: Value
  disabled?: boolean
  onChange: (value: Value) => void
}

/**
 * A scale of a few stops with a tick for each and its word under it (v0.135,
 * FR-07 7.3): how big the subject is, how much of it shows. Kilna's own
 * component rather than a change to the design system's slider, which is a
 * snippet of the line written by its tool - a slider moves along a range,
 * and these are steps with names, each one a choice.
 *
 * Read as a group of radio buttons: arrow keys walk the stops, and each stop
 * says its word.
 */
export function DiscreteScalePicker<Value extends string>({
  label,
  steps,
  value,
  disabled,
  onChange,
}: Props<Value>) {
  const at = Math.max(
    0,
    steps.findIndex((step) => step.value === value),
  )
  const walk = (by: number) => {
    const next = steps[Math.min(steps.length - 1, Math.max(0, at + by))]
    if (next !== undefined && next.value !== value) onChange(next.value)
  }
  return (
    <div
      role="radiogroup"
      aria-label={label}
      aria-disabled={disabled}
      className="relative grid gap-y-1"
      style={{ gridTemplateColumns: `repeat(${String(steps.length)}, minmax(0, 1fr))` }}
      onKeyDown={(event) => {
        if (disabled === true) return
        if (event.key === 'ArrowRight' || event.key === 'ArrowDown') {
          event.preventDefault()
          walk(1)
        } else if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') {
          event.preventDefault()
          walk(-1)
        }
      }}
    >
      {/* The track behind the ticks, from the first stop to the last. */}
      <span
        aria-hidden
        className="pointer-events-none absolute top-2 h-px bg-line-2"
        style={{
          left: `${String(50 / steps.length)}%`,
          right: `${String(50 / steps.length)}%`,
        }}
      />
      {steps.map((step, index) => {
        const on = index === at
        return (
          <Button
            key={step.value}
            variant="icon"
            size={null}
            role="radio"
            aria-checked={on}
            tabIndex={on ? 0 : -1}
            disabled={disabled}
            onClick={() => onChange(step.value)}
            className="group h-auto min-w-0 flex-col items-center justify-start gap-1 px-0.5 py-0 text-center font-normal whitespace-normal hover:bg-transparent"
          >
            <span
              aria-hidden
              className={cn(
                'relative z-[1] size-4 rounded-full border-2 transition-colors duration-quick',
                on
                  ? 'border-accent bg-accent'
                  : 'border-line-2 bg-raise group-hover:not-disabled:border-accent',
              )}
            />
            {step.icon !== undefined && (
              <span aria-hidden className={cn('text-faint', on && 'text-accent')}>
                {step.icon}
              </span>
            )}
            <span
              className={cn('text-2xs leading-tight text-dim', on && 'font-semibold text-text')}
            >
              {step.label}
            </span>
          </Button>
        )
      })}
    </div>
  )
}
