import { useTranslation } from 'react-i18next'
import type { Meta } from '@/lib/api/types'
import { useFieldDraft } from '@/components/ui/field-draft'
import { Textarea } from '@/components/ui/textarea'

/** A stored value as the text a box shows; nothing for none. */
export function textOf(value: Meta[string] | undefined): string | null {
  return value === undefined || value === null || value === false ? null : String(value)
}

/**
 * A paragraph field - a premise, an idea - read as a paragraph and written in
 * the same place.
 *
 * The box wears no border until it is pointed at, the way an InlineField sits
 * as text: on the overview a paragraph is read far more often than it is
 * rewritten. Bound by the same rules as every field (`useFieldDraft`), with
 * Enter kept for new lines: leaving the box keeps what was written, Escape
 * puts back what was there.
 */
export function MetaParagraph({
  label,
  value,
  onCommit,
}: {
  label: string
  value: Meta[string]
  onCommit: (text: string) => void
}) {
  const { t } = useTranslation()
  const draft = useFieldDraft(textOf(value) ?? '', onCommit, { multiline: true })
  return (
    <Textarea
      aria-label={label}
      placeholder={t('overview.proseEmpty')}
      autoResize
      maxRows={12}
      rows={2}
      {...draft}
      className="-ml-2 w-full resize-none border-transparent px-2 py-1 leading-relaxed text-dim hover:border-line-2 hover:bg-soft focus:bg-soft focus:text-text"
    />
  )
}
