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
 * - `list` is an array of strings, drawn as entries added and taken out one
 *   by one (v0.112): the anchors of an experiment.
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
      case 'list':
        properties[field.key] = { type: 'array', items: { type: 'string' }, title }
        break
      // How the feeling moves, section by section (v0.146): drawn by
      // kilna's own editor, a table and a curve.
      case 'arc':
        properties[field.key] = {
          type: 'array',
          items: {
            type: 'object',
            properties: {
              section: { type: 'string' },
              emotion: { type: 'string' },
              intensity: { type: 'integer', minimum: 1, maximum: 10 },
            },
          },
          title,
        }
        break
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
    if (field.type === 'list') {
      const entries = listEntries(value)
      if (entries.length > 0) record[field.key] = entries
      continue
    }
    record[field.key] = value
  }
  return record
}

/**
 * The entries of a list field's stored value: the strings of a list, or the
 * lines of a text - what the field held while it was one, as the anchors of
 * an experiment were until v0.112. The backend reads it the same way
 * (`MetaFieldType::entries`).
 */
export function listEntries(value: unknown): string[] {
  const kept = (text: string) => text.trim()
  if (Array.isArray(value)) {
    return value
      .filter((entry): entry is string => typeof entry === 'string')
      .map(kept)
      .filter((entry) => entry !== '')
  }
  if (typeof value === 'string') {
    return value
      .split(/\r?\n/)
      .map(kept)
      .filter((entry) => entry !== '')
  }
  return []
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
