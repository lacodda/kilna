import { describe, expect, it } from 'vitest'
import type { MetaField } from '@/lib/api/types'
import { editedField, metaRecord, metaSchema } from './metaSchema'

const say = (label: unknown) => (typeof label === 'string' ? label : JSON.stringify(label))

const FIELDS: MetaField[] = [
  { key: 'bpm', label: 'BPM', type: 'number' },
  { key: 'premise', label: 'Premise', type: 'multiline' },
  { key: 'recorded_on', label: 'Recorded on', type: 'date' },
  { key: 'explicit', label: 'Explicit', type: 'boolean' },
  {
    key: 'variant',
    label: 'Variant',
    type: 'choice',
    options: [
      { key: 'original', label: 'Original' },
      { key: 'slowed', label: 'Slowed' },
    ],
  },
]

describe('a work’s fields as a schema', () => {
  it('maps each type onto the schema dowel draws', () => {
    const schema = metaSchema(FIELDS, {}, say) as { properties: Record<string, object> }
    expect(schema.properties).toEqual({
      bpm: { type: 'number', title: 'BPM' },
      premise: { type: 'string', title: 'Premise', 'x-control': 'multiline' },
      recorded_on: { type: 'string', format: 'date', title: 'Recorded on' },
      explicit: { type: 'boolean', title: 'Explicit' },
      variant: {
        title: 'Variant',
        oneOf: [
          { const: 'original', title: 'Original' },
          { const: 'slowed', title: 'Slowed' },
        ],
      },
    })
  })

  it('keeps a key a choice no longer offers as an answer of its own', () => {
    const schema = metaSchema(FIELDS, { variant: 'remix' }, say) as {
      properties: { variant: { oneOf: unknown[] } }
    }
    expect(schema.properties.variant.oneOf).toContainEqual({ const: 'remix', title: 'remix' })
  })

  it('reads an empty value as an absent key, and keeps a no', () => {
    expect(
      metaRecord(FIELDS, { bpm: 120, premise: '', recorded_on: null, explicit: false }),
    ).toEqual({ bpm: 120, explicit: false })
  })

  it('names the one field an edit changed', () => {
    expect(editedField({ bpm: 120 }, { bpm: 120, explicit: true })).toEqual({
      key: 'explicit',
      value: true,
    })
    expect(editedField({ recorded_on: '2026-09-01' }, {})).toEqual({
      key: 'recorded_on',
      value: undefined,
    })
    expect(editedField({ bpm: 120 }, { bpm: 120 })).toBeUndefined()
  })
})
