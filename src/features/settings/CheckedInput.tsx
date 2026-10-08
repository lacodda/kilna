import { useState, type ReactNode } from 'react'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'

/**
 * A box that writes into the draft only what the profile would keep.
 *
 * What is typed and refused stays in the box with the reason under it, and
 * the draft keeps the last good value: a profile holding a half-typed `16:`
 * could not be saved at all, and would say so only at the bar, far from the
 * box that did it. The refused text belongs to the value it was typed over,
 * so a value that changes under it - a discard, an undo - shows through.
 */
export function CheckedInput({
  label,
  help,
  value,
  placeholder,
  className,
  check,
  onCommit,
}: {
  label: ReactNode
  help?: string
  value: string
  placeholder?: string
  className?: string
  /** What is wrong with the text, or `null` when it may be kept. */
  check: (text: string) => string | null
  onCommit: (text: string) => void
}) {
  const [refused, setRefused] = useState<{ text: string; over: string } | null>(null)
  const typed = refused !== null && refused.over === value ? refused.text : null

  return (
    <Field
      label={label}
      help={help}
      error={typed === null ? undefined : (check(typed) ?? undefined)}
    >
      <Input
        className={className}
        value={typed ?? value}
        placeholder={placeholder}
        onChange={(event) => {
          const text = event.target.value
          if (check(text) === null) {
            setRefused(null)
            onCommit(text)
          } else {
            setRefused({ text, over: value })
          }
        }}
      />
    </Field>
  )
}
