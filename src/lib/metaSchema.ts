import type { JsonSchema } from '@/components/ui/schema-shape'
import type { Label, Meta, MetaField } from '@/lib/api/types'

/**
 * A work's profile fields as the JSON Schema dowel's SchemaFields draws.
 *
 * The profile already describes each field - its key, its label, its type,
 * the answers of a choice - and the overview's form was a second description
 * of it, written by hand (`MetaInput` until v0.110). A list of fields maps
 * onto a schema object one to one (wish 5051):
 *
 * - `text` and `number` are strings and numbers. The overview draws them
 *   itself as InlineFields (`custom`): read far more often than edited, and
 *   written once on Enter rather than on every keystroke.
 * - `multiline` is a string drawn as a paragraph (`x-control: multiline`).
 * - `date` is a string in the `date` format, which a DatePicker draws, with a
 *   way to clear it - none of the fields is required.
 * - `boolean` is a boolean, a Switch beside its name.
 * - `choice` is a union of `const`s, each titled by its label in the
 *   language the window speaks. A key the field no longer offers stays as an
 *   answer of its own, titled by itself, rather than the picker pretending
 *   nothing was chosen.
 */
export function metaSchema(
  fields: readonly MetaField[],
  meta: Readonly<Meta>,
  say: (label: Label) => string,
): JsonSchema {
  const properties: Record<string, JsonSchema> = {}
  for (const field of fields) {
    const title = say(field.label)
    switch (field.type) {
      case 'text':
        properties[field.key] = { type: 'string', title }
        break
      case 'multiline':
        properties[field.key] = { type: 'string', title, 'x-control': 'multiline' }
        break
      case 'number':
        properties[field.key] = { type: 'number', title }
        break
      case 'date':
        properties[field.key] = { type: 'string', format: 'date', title }
        break
      case 'boolean':
        properties[field.key] = { type: 'boolean', title }
        break
      case 'choice': {
        const answers = (field.options ?? []).map((option) => ({
          const: option.key,
          title: say(option.label),
        }))
        const chosen = meta[field.key]
        if (
          typeof chosen === 'string' &&
          chosen !== '' &&
          !answers.some((answer) => answer.const === chosen)
        ) {
          answers.push({ const: chosen, title: chosen })
        }
        properties[field.key] = { title, oneOf: answers }
        break
      }
    }
  }
  return { type: 'object', properties }
}

/**
 * The record SchemaFields edits, read off a work's fields: only the fields
 * the schema has, and none that is empty - an empty value is an absent key,
 * the way SchemaFields writes one.
 */
export function metaRecord(
  fields: readonly MetaField[],
  meta: Readonly<Meta>,
): Record<string, unknown> {
  const record: Record<string, unknown> = {}
  for (const field of fields) {
    const value = meta[field.key]
    if (value === undefined || value === null || value === '') continue
    record[field.key] = value
  }
  return record
}

/**
 * The one field that differs between two records, and its new value - what
 * SchemaFields' next record says was edited. `undefined` when nothing did.
 */
export function editedField(
  before: Readonly<Record<string, unknown>>,
  after: Readonly<Record<string, unknown>>,
): { key: string; value: unknown } | undefined {
  for (const key of new Set([...Object.keys(before), ...Object.keys(after)])) {
    if (!Object.is(before[key], after[key])) return { key, value: after[key] }
  }
  return undefined
}
