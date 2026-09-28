import type { Label, Meta, ReleaseFieldValue } from '@/lib/api/types'

/*
 * How full a release's metadata is.
 *
 * Its own module rather than a line inside the component, because the same
 * answer is wanted in two places that cannot share a render: the boxes on the
 * Releases tab, and the calendar deciding which releases a batch is worth
 * running over. A second copy of "what counts as written" would drift, and
 * the two screens would disagree about whether a release is ready.
 */

export interface Written {
  written: number
  total: number
  /** Every field the kind asks for has something in it. */
  complete: boolean
}

/**
 * What of these fields is written.
 *
 * Blank is unwritten: a box holding three spaces is a box nobody filled, and
 * counting it would let a release call itself ready on whitespace. A kind
 * that asks for nothing is complete - nothing was asked of it.
 */
export function written(fields: ReleaseFieldValue[]): Written {
  const written = fields.filter((field) => field.value.trim() !== '').length
  return { written, total: fields.length, complete: written === fields.length }
}

/**
 * Whether the profile can fill any of these fields on its own.
 *
 * What the generate button hangs on: a kind whose every field is typed by
 * hand should not offer one, because pressing it would do nothing and teach
 * the person that it does nothing.
 */
export function fillable(fields: ReleaseFieldValue[]): boolean {
  return fields.some((field) => field.has_template)
}

/**
 * How far past its limit a field is, or null when nothing is counting.
 *
 * A limit is counted, never enforced: kilna is not the authority on what a
 * platform accepts this month, and a box that refuses to hold what someone
 * typed is a box they type somewhere else instead.
 */
export function over(field: ReleaseFieldValue, value: string): number | null {
  if (field.limit == null) return null
  const excess = value.length - field.limit
  return excess > 0 ? excess : null
}

/** One field a generation would rewrite: what it holds, and what it would. */
export interface Replacement {
  key: string
  label: Label
  before: string
  after: string
}

/**
 * What writing a release's fields from its work would change, field by field.
 *
 * Only what differs. A field the templates would write back word for word
 * has nothing to show, and a preview that lists it beside the ones that do
 * change hides the one change in a page of sameness. A field with no
 * rendered value is left alone by the generation, so it is not listed
 * either; what the renderer refused is reported beside, not here.
 */
export function replacements(fields: ReleaseFieldValue[], values: Meta): Replacement[] {
  const out: Replacement[] = []
  for (const field of fields) {
    if (!Object.hasOwn(values, field.key)) continue
    const rendered = values[field.key]
    const after = typeof rendered === 'string' ? rendered : JSON.stringify(rendered)
    if (after === field.value) continue
    out.push({ key: field.key, label: field.label, before: field.value, after })
  }
  return out
}

/**
 * Which of these releases a batch is worth running over.
 *
 * What is already out is left alone: its metadata is a record of what went
 * out under, and rewriting it from today's lyrics would quietly rewrite
 * history. Everything still planned is fair game, including what already has
 * text in it — replacing that is exactly what the batch was asked to do, and
 * the dialog in front of it says so.
 */
export function batchable<T extends { id: string; status: string }>(releases: T[]): string[] {
  return releases.filter((release) => release.status !== 'released').map((release) => release.id)
}
