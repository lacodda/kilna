import { useState, type Ref } from 'react'
import { cn, useLocale } from 'dowel-ui'
import { fieldClasses } from './input'

/*
 * TimeField - a time of day, typed the way people say it.
 *
 * No donor for this one: neither product of the line had a time field, so
 * this is written from the same shape as DurationField, and for the same
 * reason. Anything a person plausibly types is accepted - `9`, `9:30`, `930`,
 * `9.30`, `9pm`, `21:30` - and what comes back is always `HH:MM`.
 *
 * The value is `HH:MM` in twenty-four hours, always, no matter how it was
 * typed or how it is shown. That is what a database column holds and what
 * sorts correctly as a string; whether the reader sees `9:30 PM` or `21:30`
 * is a matter of where they live, and `Intl` answers it.
 *
 * Not `<input type="time">`, and the reason is the same as NumberField's: the
 * browser draws its own control, its own spinner and its own clock popup,
 * none of which a stylesheet reaches - so a form of the product's own fields
 * gets one that is visibly not.
 *
 * Empty is `null`, like the other fields here: no time is not midnight.
 */

/** Minutes since midnight from whatever was typed, or `null` for empty, or
 * `undefined` when it cannot be read as a time.
 *
 * Exported because the parsing is the component - a test that types into the
 * box checks React's state handling, and what has to be right is this. */
export function parseTime(text: string): string | null | undefined {
  const input = text.trim().toLowerCase().replace(/\s+/g, '')
  if (input === '') return null

  // `pm` means add twelve hours, `am` means midnight is 12. Stripped first so
  // the rest of the parsing does not have to know about them.
  const meridiem = /(am|pm)$/.exec(input)?.[1]
  const body = meridiem ? input.slice(0, -2) : input

  let hours: number
  let minutes: number

  const separated = /^(\d{1,2})[:.](\d{2})$/.exec(body)
  if (separated) {
    hours = Number(separated[1])
    minutes = Number(separated[2])
  } else if (/^\d{1,2}$/.test(body)) {
    // A bare number is an hour: `9` is nine o'clock, not nine minutes past
    // midnight - which is what someone typing a time means.
    hours = Number(body)
    minutes = 0
  } else if (/^\d{3,4}$/.test(body)) {
    // `930` and `0930`, which is how a time gets typed when the colon is a
    // reach on a phone keyboard.
    hours = Number(body.slice(0, body.length - 2))
    minutes = Number(body.slice(-2))
  } else {
    return undefined
  }

  if (minutes > 59) return undefined

  if (meridiem) {
    if (hours < 1 || hours > 12) return undefined
    if (meridiem === 'pm' && hours !== 12) hours += 12
    if (meridiem === 'am' && hours === 12) hours = 0
  } else if (hours > 23) {
    return undefined
  }

  return `${String(hours).padStart(2, '0')}:${String(minutes).padStart(2, '0')}`
}

/** How a time reads here: `21:30` in most of the world, `9:30 PM` in some of
 * it. The stored value does not change - only what is shown. */
export function formatTime(time: string, locale: string): string {
  const [hours, minutes] = time.split(':').map(Number) as [number, number]
  return new Intl.DateTimeFormat(locale, { hour: 'numeric', minute: '2-digit' }).format(
    new Date(2024, 0, 1, hours, minutes),
  )
}

export interface TimeFieldProps {
  /** `HH:MM` in twenty-four hours, or `null` for empty. */
  value: string | null
  /** Told the next time as `HH:MM`, or `null` when the box is cleared. Fires
   * on blur and on Enter, not on every key, and not when the text parses to
   * the time already held. */
  onValueChange: (value: string | null) => void
  /** How the time is shown while the field is not being typed into. The
   * application's language unless stated - see `useLocale`. */
  locale?: string
  /** What the empty box says. Pass the product's own words. */
  placeholder?: string
  /** Stops the box taking focus or input. Passed to the `<input>`. */
  disabled?: boolean
  /** The box can be focused and read but not typed into. Passed to the
   * `<input>`. */
  readOnly?: boolean
  /** Marks the `<input>` as required for a form. */
  required?: boolean
  /** The `<input>`'s name, for a form. */
  name?: string
  /** The `<input>`'s id, so a label can point at it. */
  id?: string
  /** Reaches the `<input>`, for focusing it or reading its selection. */
  ref?: Ref<HTMLInputElement>
  /** Names the box for a screen reader when no visible label does. */
  'aria-label'?: string
  /** The id of the element that describes the box, such as a hint. */
  'aria-describedby'?: string
  className?: string
}

export function TimeField({
  value,
  onValueChange,
  locale,
  className,
  ref,
  ...props
}: TimeFieldProps) {
  const language = useLocale(locale)
  const display = (time: string | null) => (time === null ? '' : formatTime(time, language))

  /* Text while it is being typed, a formatted time the rest of the time -
   * the same arrangement as DurationField, and for the same reason: a field
   * that reformats on every keystroke fights the person using it. */
  const [text, setText] = useState(() => display(value))
  const [editing, setEditing] = useState(false)

  /* The value this box last saw from outside, adjusted during render rather
   * than in an effect. Comparing against `value` would be wrong in exactly
   * the case that matters: after a commit the parent may still hold the old
   * one for a tick, and the box would clear itself under the reader. */
  const [seen, setSeen] = useState<string | null>(value)

  if (value !== seen) {
    setSeen(value)
    if (!editing) setText(display(value))
  }

  const commit = () => {
    setEditing(false)
    const parsed = parseTime(text)
    if (parsed === undefined) {
      // Unreadable: put back what the value actually is rather than leaving
      // the box saying something the form does not believe.
      setText(display(value))
      return
    }
    setText(display(parsed))
    if (parsed !== value) onValueChange(parsed)
  }

  return (
    <input
      {...props}
      ref={ref}
      type="text"
      inputMode="numeric"
      value={text}
      onFocus={() => setEditing(true)}
      onChange={(event) => setText(event.target.value)}
      onBlur={commit}
      onKeyDown={(event) => {
        if (event.key === 'Enter') {
          event.preventDefault()
          commit()
        }
      }}
      className={cn(fieldClasses, 'h-control tabular-nums', className)}
    />
  )
}
