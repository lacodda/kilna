import { useId, type ReactNode } from 'react'
import { cn } from 'dowel-ui'
import { useFieldDraft } from './field-draft'

/*
 * InlineField.
 *
 * A value that is edited where it is shown: click it, type, Enter or leave to
 * keep, Escape to take it back.
 *
 * A board of fields - the numbers on a work's overview, the cells of a
 * storyboard - is read far more often than it is edited, so every value there
 * sits as text until it is touched. A modal to change one BPM is a lost
 * context and a click too many, and a column of always-on boxes is a form
 * nobody asked to fill in. kilna had nine of these editors, each with its own
 * idea of what Enter and Escape mean; this is one idea, and its rules are
 * FieldDraft's.
 *
 * **It is always an `<input>`.** Not text that swaps to an input on click: a
 * swap loses the caret position, steals a render, and leaves the value
 * unreachable by Tab. The box is simply undressed until it is hovered or
 * focused - the mockup's `.inl .v`: a border appears under the pointer, the
 * accent when you are in it.
 *
 * **What a value is, is a codec.** `parse` reads what was typed and `format`
 * spells the stored value, so the box can never show one thing and store
 * another. Text needs neither. `numberCodec` and `timecodeCodec` are the two
 * the line reached for; a product's own - a key signature, a tempo range - is
 * two functions. `parse` answers `null` for an empty box (a value cleared,
 * not a value of zero) and `undefined` for text it cannot read, which the
 * field refuses rather than guesses at.
 */

export interface Codec<T> {
  /** The value in the text, `null` for none, `undefined` when it cannot be
   * read. */
  parse: (text: string) => T | null | undefined
  /** The one spelling the value is shown in. */
  format: (value: T) => string
  /** Set in the monospaced face with tabular figures, so a column of them
   * lines up. */
  mono?: boolean
  /** The keyboard a phone offers. */
  inputMode?: 'text' | 'decimal' | 'numeric'
}

/** Text as it is typed. Nothing to read, so nothing to refuse. */
export const textCodec: Codec<string> = {
  parse: (text) => text,
  format: (value) => value,
}

/** A number, with a comma accepted as the decimal point - half the world
 * writes one. Empty is `null`, and anything else that is not a number is
 * refused. */
export const numberCodec: Codec<number> = {
  parse: (text) => {
    const trimmed = text.trim().replace(',', '.')
    if (trimmed === '') return null
    if (!/^-?\d+(?:\.\d+)?$/.test(trimmed)) return undefined
    return Number(trimmed)
  },
  format: (value) => String(value),
  mono: true,
  inputMode: 'decimal',
}

/**
 * Seconds as a person types them on a timeline: `83`, `1:23`, `0:04.5`,
 * `1:00:00`. A minute of seventy-five seconds is a typo and is refused, not
 * read as a minute and a quarter. Written back as `m:ss`, with a tenth only
 * when there is one - hours are not split out, because a clip is minutes long
 * and `60:00` reads better beside `0:04` than `1:00:00` does.
 */
export const timecodeCodec: Codec<number> = {
  parse: (text) => {
    const trimmed = text.trim()
    if (trimmed === '') return null
    const parts = trimmed.split(':')
    if (parts.length > 3) return undefined
    let seconds = 0
    for (const [index, part] of parts.entries()) {
      // Only the last part may carry a fraction; the others are whole units.
      const last = index === parts.length - 1
      if (!(last ? /^\d+(?:\.\d+)?$/ : /^\d+$/).test(part)) return undefined
      const amount = Number(part)
      if (index > 0 && amount >= 60) return undefined
      seconds = seconds * 60 + amount
    }
    return seconds
  },
  format: (seconds) => {
    if (!Number.isFinite(seconds) || seconds < 0) return ''
    // Rounded to a tenth before splitting, so 59.96 does not read as 0:60.0.
    const tenths = Math.round(seconds * 10)
    const minutes = Math.floor(tenths / 600)
    const rest = tenths - minutes * 600
    const whole = String(Math.floor(rest / 10)).padStart(2, '0')
    const fraction = rest % 10
    return fraction === 0 ? `${minutes}:${whole}` : `${minutes}:${whole}.${fraction}`
  },
  mono: true,
}

interface InlineFieldBaseProps {
  /** What the value is called: the caption above it. */
  label: ReactNode
  /** Keep the caption for a screen reader and take it off the screen - for a
   * cell whose column header already names it. */
  labelHidden?: boolean
  /** What an empty value shows. A dash reads as "nothing yet"; a blank box
   * reads as a rendering fault. */
  placeholder?: string
  disabled?: boolean
  className?: string
}

export interface InlineFieldProps<T> extends InlineFieldBaseProps {
  /** The stored value; `null` for none. */
  value: T | null
  /** Called once, with the parsed value, when a change is kept. Return
   * `false` to refuse it - a server that said no - and the field goes back
   * to the stored value. */
  onCommit: (value: T | null) => boolean | void
  codec: Codec<T>
}

export function InlineField(
  props: InlineFieldBaseProps & {
    value: string | null
    onCommit: (value: string | null) => boolean | void
    codec?: Codec<string>
  },
): ReactNode
export function InlineField<T>(props: InlineFieldProps<T>): ReactNode
export function InlineField<T>({
  label,
  labelHidden = false,
  placeholder,
  disabled = false,
  className,
  value,
  onCommit,
  // Text is the default, and only for a value that is text: the overloads
  // above refuse a number without a codec at compile time.
  codec = textCodec as unknown as Codec<T>,
}: Omit<InlineFieldProps<T>, 'codec'> & { codec?: Codec<T> }) {
  const id = useId()
  const stored = value === null ? '' : codec.format(value)

  const draft = useFieldDraft(
    stored,
    (text) => {
      const parsed = codec.parse(text)
      // `readable` has already turned away `undefined`; this is the type
      // system's copy of the same fact.
      if (parsed === undefined) return false
      return onCommit(parsed)
    },
    { readable: (text) => codec.parse(text) !== undefined },
  )

  return (
    <div className={cn('flex min-w-0 flex-col gap-0.5', className)}>
      <label htmlFor={id} className={cn('caption', labelHidden && 'sr-only')}>
        {label}
      </label>
      <input
        id={id}
        type="text"
        disabled={disabled}
        placeholder={placeholder}
        inputMode={codec.inputMode}
        autoComplete="off"
        spellCheck={codec.mono ? false : undefined}
        {...draft}
        className={cn(
          // Pulled left by its own padding, so the text lines up with the
          // caption above it and with plain text beside it - the box is
          // there only when it is wanted.
          '-ml-1.5 h-control-sm w-full min-w-0 rounded-sm border border-transparent bg-transparent px-1.5',
          'text-sm text-ellipsis text-text placeholder:text-faint',
          'cursor-text transition-colors outline-none',
          'hover:border-line-2 hover:bg-soft',
          'focus:border-accent focus:bg-soft',
          'aria-invalid:border-bad aria-invalid:focus:border-bad',
          'disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:border-transparent disabled:hover:bg-transparent',
          codec.mono && 'font-mono tabular-nums',
        )}
      />
    </div>
  )
}
