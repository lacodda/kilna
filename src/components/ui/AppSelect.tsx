import { Check } from 'lucide-react'
import {
  Select as Base,
  SelectItem,
  SelectItemIndicator,
  SelectItemText,
  SelectPopup,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'

/*
 * Casing here is the convention, not a style: a lowercase file in
 * `components/ui/` is a copy from the registry and is never edited, so it can
 * stay byte-identical to upstream; a PascalCase one is this app's own, and
 * this is one of those.
 */

/*
 * kilna's own shape of a select.
 *
 * dowel exposes the parts; every place in this app that picks one of a short
 * list has the same shape - a flat array of `{ value, label }` and an optional
 * "any" entry at the top - so the shape lives here rather than at nine call
 * sites.
 *
 * The empty entry is the reason this wrapper is not just a convenience. A
 * value of `''` means "no filter", and an item cannot carry an empty value, so
 * it travels as a sentinel and is unwrapped on both sides of the boundary.
 * Every caller doing that by hand is nine chances to do it differently.
 */

export interface Option {
  value: string
  label: string
}

interface Props {
  value: string
  onChange: (value: string) => void
  options: Option[]
  /** Entry shown when nothing is chosen; picking it yields an empty string. */
  placeholder?: string
  className?: string
  'aria-label'?: string
}

/** Base UI items may not carry an empty value, so the placeholder entry is
 * smuggled through a sentinel and unwrapped on both sides. */
const EMPTY = ' empty'

export function Select({
  value,
  onChange,
  options,
  placeholder,
  className,
  'aria-label': ariaLabel,
}: Props) {
  const entries =
    placeholder === undefined ? options : [{ value: EMPTY, label: placeholder }, ...options]

  // `items` is what makes the trigger show the label rather than the raw
  // value - without it a filter reads `draft` where it should read `Draft`.
  const items = Object.fromEntries(entries.map((option) => [option.value, option.label]))

  return (
    <Base
      items={items}
      value={value === '' ? EMPTY : value}
      onValueChange={(next: unknown) => onChange(next === EMPTY ? '' : String(next))}
    >
      <SelectTrigger aria-label={ariaLabel} className={className}>
        <SelectValue placeholder={placeholder} />
      </SelectTrigger>

      <SelectPopup>
        {entries.map((option) => (
          <SelectItem key={option.value} value={option.value}>
            <SelectItemText>{option.label}</SelectItemText>
            <SelectItemIndicator className="ml-auto">
              <Check aria-hidden className="size-3.5" />
            </SelectItemIndicator>
          </SelectItem>
        ))}
      </SelectPopup>
    </Base>
  )
}
