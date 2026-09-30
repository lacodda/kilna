import i18n from '@/i18n'
import type { Meta, MetaField } from '@/lib/api/types'
import { say } from '@/lib/useProfile'

/**
 * A work's field as the words a screen shows for it; `null` for nothing.
 *
 * A choice is stored by its answer's key - `sped-up` - and read by the
 * answer's label, "Sped up", in the window's language: the key is the
 * machine's name for the answer, the same rule `labelOf` keeps for the rest
 * of the vocabulary. A key the field no longer offers is shown as it is, so a
 * value an answer left behind when it was removed stays visible rather than
 * turning blank.
 *
 * One reading for every place that shows a value it does not edit - the
 * header's strip, a proposal's summary - so a variant cannot read "Slowed" on
 * the overview and `slowed` above it.
 */
export function fieldText(field: MetaField, value: Meta[string] | undefined): string | null {
  if (value === undefined || value === null || value === '') return null
  if (field.type === 'boolean') return i18n.t(value === true ? 'work.yes' : 'work.no')
  if (field.type === 'choice') {
    const option = (field.options ?? []).find((entry) => entry.key === value)
    return option === undefined ? String(value) : say(option.label)
  }
  return typeof value === 'object' ? JSON.stringify(value) : String(value)
}
