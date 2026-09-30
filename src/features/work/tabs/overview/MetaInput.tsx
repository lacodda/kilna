import { useTranslation } from 'react-i18next'
import type { Meta, MetaField } from '@/lib/api/types'
import { say as sayLabel } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Select } from '@/components/AppSelect'
import { DatePicker } from '@/components/DatePicker'
import { FieldGroup } from '@/components/ui/field'
import { Checkbox } from '@/components/ui/checkbox'
import { useFieldDraft } from '@/components/ui/field-draft'
import { InlineField, numberCodec } from '@/components/ui/inline-field'
import { Textarea } from '@/components/ui/textarea'

interface Props {
  field: MetaField
  value: Meta[string]
  onChange: (value: Meta[string]) => void
  /** The caption beside the value rather than over it: a line of the sheet. */
  inline?: boolean
}

/**
 * One profile-defined field, edited where it is read.
 *
 * A word or a number is an InlineField: read far more often than it is
 * edited, so it sits as text until it is touched - click, Enter keeps it,
 * Escape puts it back (the owner's rule for the overview: editing happens in
 * place, and a dialog for one BPM is a click and a context lost). It follows
 * the stored value while nobody is typing and writes only a change. The
 * fields were once uncontrolled boxes that wrote on every blur, which is how
 * a value a plugin had just written was put back to the old one by the next
 * pass of the Tab key, and how tabbing through twelve fields left twelve
 * operations and twelve toasts behind.
 *
 * An empty value shows a dash: the widget around the fields is the
 * invitation, and one field of twelve with nothing in it is a fact, not a
 * call to action.
 */
export function MetaInput({ field, value, onChange, inline = false }: Props) {
  const { t } = useTranslation()
  const label = sayLabel(field.label)

  if (field.type === 'boolean') {
    // The words beside the box are its name, and the whole row is what a
    // pointer aims at - the checkbox carries its label itself.
    return (
      <Checkbox checked={value === true} onCheckedChange={(checked) => onChange(checked)}>
        {label}
      </Checkbox>
    )
  }

  if (field.type === 'date') {
    // A group rather than a Field: the picker is a button and a popup, not an
    // input a Field could hand its id to - so it carries its own name.
    return (
      <FieldGroup label={label}>
        <DatePicker
          aria-label={label}
          className="w-full"
          placeholder={t('work.noDate')}
          value={typeof value === 'string' ? value : ''}
          onChange={(next) => onChange(next)}
        />
      </FieldGroup>
    )
  }

  if (field.type === 'multiline') {
    return <MetaParagraph label={label} value={value} onCommit={(text) => onChange(text)} />
  }

  if (field.type === 'choice') {
    // One of the answers the profile offers, stored by its key and shown by
    // its label; the empty entry at the top takes the answer back. A picker
    // rather than a box: typing "slowed" by hand is how a variant ends up
    // spelt three ways across three releases.
    const chosen = textOf(value) ?? ''
    const options = (field.options ?? []).map((option) => ({
      value: option.key,
      label: sayLabel(option.label),
    }))
    // A key the field no longer offers stays visible as it is, rather than
    // the picker pretending nothing was chosen.
    if (chosen !== '' && !options.some((option) => option.value === chosen)) {
      options.push({ value: chosen, label: chosen })
    }
    return (
      <FieldGroup label={label}>
        <Select
          aria-label={label}
          className="h-control-sm w-full text-xs"
          value={chosen}
          placeholder={t('fields.choiceNone')}
          onChange={(next) => onChange(next)}
          options={options}
        />
      </FieldGroup>
    )
  }

  // The caption at a fixed width to the left, right-aligned, on a line of the
  // sheet - the mockup's `.inline-fields`, where six fields read as one line
  // of label and value pairs rather than a grid. The value at a fixed width
  // too, so the pairs fall into a rhythm instead of each taking what its
  // box happens to measure.
  const beside =
    inline &&
    'flex-row items-center gap-2 [&>input]:w-28 [&>label]:w-22 [&>label]:shrink-0 [&>label]:text-right'

  if (field.type === 'number') {
    // Numbers are stored as numbers so scoring and sorting can use them; what
    // cannot be read as one is refused by the field rather than stored as
    // text. A number a plugin wrote as text is still shown as the number.
    const stored =
      typeof value === 'number'
        ? value
        : typeof value === 'string'
          ? numberCodec.parse(value)
          : null
    return (
      <InlineField
        label={label}
        placeholder="—"
        codec={numberCodec}
        value={stored ?? null}
        onCommit={(next) => onChange(next ?? '')}
        className={cn(beside)}
      />
    )
  }

  return (
    <InlineField
      label={label}
      placeholder="—"
      value={textOf(value)}
      onCommit={(next) => onChange(next ?? '')}
      className={cn(beside)}
    />
  )
}

/** A stored value as the text a box shows; nothing for none. */
function textOf(value: Meta[string]): string | null {
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
