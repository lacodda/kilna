import { useTranslation } from 'react-i18next'
import type { Label } from '@/lib/api/types'
import { relabel, relabelOptional, resolveLabel } from '@/lib/label'
import { Input } from '@/components/ui/input'

/**
 * A word of the profile, edited in the language the window speaks.
 *
 * A shipped word is kept in two languages, and the box shows and rewrites the
 * one the window is in: the other stays as it was, so a Russian window that
 * renames a status does not take the English word away from an English one.
 * A word the person typed is one string and stays one.
 */
export function LabelInput({
  label,
  onChange,
  optional = false,
  ...props
}: {
  label: Label | null | undefined
  /** `undefined` only from an `optional` box that was emptied. */
  onChange: (label: Label | undefined) => void
  /** The word may be absent - a description, a hint: emptied, it is none. */
  optional?: boolean
  className?: string
  placeholder?: string
  'aria-label'?: string
}) {
  const { i18n } = useTranslation()
  const language = i18n.resolvedLanguage ?? 'en'

  return (
    <Input
      {...props}
      value={resolveLabel(label, language)}
      onChange={(event) => {
        const text = event.target.value
        onChange(optional ? relabelOptional(label, text, language) : relabel(label, text, language))
      }}
    />
  )
}
