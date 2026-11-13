import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import type { Meta, MetaField } from '@/lib/api/types'
import { editedField, metaRecord, metaSchema } from '@/lib/metaSchema'
import { say as sayLabel } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { useSchemaWords } from '@/components/useSchemaWords'
import { Button } from '@/components/ui/button'
import { InlineField, numberCodec } from '@/components/ui/inline-field'
import { SchemaFields, type SchemaCustomProps } from '@/components/ui/schema-fields'
import { textOf } from '@/features/work/tabs/overview/MetaParagraph'

interface Props {
  /** The profile's fields of the work's kind, in the order written. */
  fields: readonly MetaField[]
  meta: Meta
  /** One field changed: its key, and its value - empty for none. */
  onChange: (key: string, value: Meta[string]) => void
  /** The caption beside the value rather than over it: a line of the sheet. */
  inline?: boolean
  /** Give the work the field's next code - offered while a numbered field
   *  is empty (ADR 0059). */
  onNumber?: (key: string) => void
  /** A code is on its way; the offer waits for it. */
  numbering?: boolean
}

/**
 * The profile's fields of a work, edited where they are read: dowel's
 * SchemaFields over the schema the profile's fields describe (`metaSchema`,
 * wish 5051) - a Select for a choice, a DatePicker that can clear its date, a
 * Switch for a yes or no, each in a Field that names it.
 *
 * A word or a number is kilna's own drawing (`custom`): an InlineField, read
 * far more often than it is edited, so it sits as text until it is touched -
 * click, Enter keeps it, Escape puts it back (the owner's rule for the
 * overview: a dialog for one BPM is a click and a context lost). It writes
 * only a change, once, rather than on every keystroke as a schema's box does.
 *
 * Drawn `contents`, so the fields stand in the grid of the widget around them,
 * beside the status and the kind. An empty word shows a dash: the widget is
 * the invitation, and one field of twelve with nothing in it is a fact.
 */
export function WorkFields({
  fields,
  meta,
  onChange,
  inline = false,
  onNumber,
  numbering = false,
}: Props) {
  const { i18n } = useTranslation()
  const words = useSchemaWords()
  // The labels are said in the window's language.
  const language = i18n.resolvedLanguage
  const schema = useMemo(
    () => metaSchema(fields, meta, (label) => sayLabel(label, language)),
    [fields, meta, language],
  )
  const value = metaRecord(fields, meta)

  const custom = Object.fromEntries(
    fields
      .filter((field) => field.type === 'text' || field.type === 'number')
      .map((field) => [
        field.key,
        (props: SchemaCustomProps) => (
          <InlineMeta
            {...props}
            meta={field}
            inline={inline}
            onNumber={onNumber === undefined ? undefined : () => onNumber(field.key)}
            numbering={numbering}
          />
        ),
      ]),
  )

  return (
    <SchemaFields
      className="contents"
      schema={schema}
      value={value}
      onValueChange={(next) => {
        const edit = editedField(value, next)
        if (edit !== undefined) onChange(edit.key, (edit.value ?? null) as Meta[string])
      }}
      words={words}
      custom={custom}
    />
  )
}

/** A word or a number of the work's, as an InlineField. */
function InlineMeta({
  field,
  value,
  onValueChange,
  meta,
  inline,
  onNumber,
  numbering,
}: SchemaCustomProps & {
  meta: MetaField
  inline: boolean
  onNumber?: () => void
  numbering: boolean
}) {
  const { t } = useTranslation()
  const stored = value as Meta[string] | undefined

  // The caption at a fixed width to the left, right-aligned, on a line of the
  // sheet - the mockup's `.inline-fields`, where six fields read as one line
  // of label and value pairs rather than a grid. The value at a fixed width
  // too, so the pairs fall into a rhythm instead of each taking what its box
  // happens to measure.
  const beside =
    inline &&
    'flex-row items-center gap-2 [&>input]:w-28 [&>label]:w-22 [&>label]:shrink-0 [&>label]:text-right'

  if (meta.type === 'number') {
    // Numbers are stored as numbers so scoring and sorting can use them; what
    // cannot be read as one is refused by the field rather than stored as
    // text. A number a plugin wrote as text is still shown as the number.
    const number =
      typeof stored === 'number'
        ? stored
        : typeof stored === 'string'
          ? numberCodec.parse(stored)
          : null
    return (
      <InlineField
        label={field.label}
        placeholder="—"
        codec={numberCodec}
        value={number ?? null}
        onCommit={(next) => onValueChange(next ?? undefined)}
        className={cn(beside)}
      />
    )
  }

  const text = textOf(stored)
  const box = (
    <InlineField
      label={field.label}
      placeholder="—"
      value={text}
      onCommit={(next) => onValueChange(next === null || next === '' ? undefined : next)}
      className={cn(beside)}
    />
  )
  // A numbered field nobody has filled offers its next code under the box,
  // the way a pinned status offers the way back: typed by hand, a code is
  // the one the person remembered rather than the next one free.
  if (onNumber === undefined || meta.numbered_from == null || (text ?? '').trim() !== '') {
    return box
  }
  return (
    <div className="flex min-w-0 flex-col gap-0.5">
      {box}
      <span className="text-xs">
        <Button
          variant="link"
          disabled={numbering}
          title={t('fields.giveCodeHint', { field: field.label })}
          onClick={onNumber}
        >
          {t('fields.giveCode')}
        </Button>
      </span>
    </div>
  )
}
