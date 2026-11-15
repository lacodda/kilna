import type { ReactNode } from 'react'
import { Checkbox } from '@/components/ui/checkbox'

/**
 * A handful of boxes to tick, for a filter over a list of keys.
 *
 * The set's Checkbox, as the rows use: three to eight of them in a panel
 * need no list navigation, and the same box in the panel and on the row
 * reads as the same kind of control.
 */
export function CheckList<T extends string | number>({
  options,
  chosen,
  onChange,
}: {
  /** `icon` is drawn before the label, for a column the rows draw as a picture
   * rather than as a word - the filter then offers what the column shows. */
  options: { value: T; label: string; icon?: ReactNode }[]
  chosen: readonly T[]
  onChange: (next: T[]) => void
}) {
  return (
    <>
      {options.map((option) => {
        const on = chosen.includes(option.value)
        return (
          <Checkbox
            key={String(option.value)}
            checked={on}
            onCheckedChange={() =>
              onChange(
                on ? chosen.filter((value) => value !== option.value) : [...chosen, option.value],
              )
            }
            className="px-1 py-0.5"
          >
            {option.icon}
            <span className="truncate">{option.label}</span>
          </Checkbox>
        )
      })}
    </>
  )
}
