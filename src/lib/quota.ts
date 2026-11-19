import type { JsonValue } from '@/lib/api/generated/serde_json/JsonValue'
import type { ProfileConfig, QuotaMetric, QuotaScope, QuotaSeverity } from '@/lib/api/types'

/*
 * A rule of a quotas section as the editor holds it (v0.151, FR-20 20.2):
 * the fact's data spread over the boxes a person fills, and back. The
 * backend checks the whole rule when it is written (`canon::quota`); what is
 * here only keeps the boxes and the data in step.
 */

export const METRICS: readonly QuotaMetric[] = [
  'char_len',
  'word_count',
  'count_per_value',
  'unique_count',
  'match_count',
  'share',
  'forbidden_terms',
]

export const SCOPES: readonly QuotaScope[] = ['item', 'collection_period', 'global']

/** The comparisons a measure takes; the words a text must not say take none. */
export type QuotaOp = 'between' | 'min' | 'max' | 'eq'
export const OPS: readonly QuotaOp[] = ['between', 'min', 'max', 'eq']

export interface QuotaDraft {
  field: string
  metric: QuotaMetric
  op: QuotaOp
  /** The number of min, max and eq, and the low end of between. */
  low: number | null
  high: number | null
  /** The words of forbidden_terms, one a line or comma by comma. */
  terms: string
  pattern: string
  scope: QuotaScope
  severity: QuotaSeverity
  active: boolean
  filterField: string
  /** The values the filter keeps, comma by comma. */
  filterEquals: string
  filterNot: boolean
}

const text = (value: JsonValue | undefined) => (typeof value === 'string' ? value : '')
const figure = (value: JsonValue | undefined) => (typeof value === 'number' ? value : null)
const words = (line: string) =>
  line
    .split(/[,\n]/)
    .map((one) => one.trim())
    .filter((one) => one !== '')

/** The boxes of a rule written down - or of a new one. */
export function draftOf(data: Record<string, JsonValue | undefined>): QuotaDraft {
  const metric = (METRICS as readonly string[]).includes(text(data.metric))
    ? (text(data.metric) as QuotaMetric)
    : 'char_len'
  const op = (OPS as readonly string[]).includes(text(data.op)) ? (text(data.op) as QuotaOp) : 'max'
  const value = data.value
  const pair = Array.isArray(value) ? value : null
  const filter =
    data.filter !== null && typeof data.filter === 'object' && !Array.isArray(data.filter)
      ? data.filter
      : null
  const equals = filter?.equals
  return {
    field: text(data.field),
    metric,
    op,
    low: metric === 'forbidden_terms' ? null : pair !== null ? figure(pair[0]) : figure(value),
    high: pair !== null && metric !== 'forbidden_terms' ? figure(pair[1]) : null,
    terms:
      metric === 'forbidden_terms' && pair !== null
        ? pair.filter((one): one is string => typeof one === 'string').join(', ')
        : '',
    pattern: text(data.pattern),
    scope: (SCOPES as readonly string[]).includes(text(data.scope))
      ? (text(data.scope) as QuotaScope)
      : 'item',
    severity: data.severity === 'error' ? 'error' : 'warn',
    active: data.active !== false,
    filterField: text(filter?.field),
    filterEquals: Array.isArray(equals)
      ? equals.filter((one): one is string => typeof one === 'string').join(', ')
      : text(equals),
    filterNot: filter?.not === true,
  }
}

/** The data a rule is written with, from its boxes. */
export function dataOf(draft: QuotaDraft): Record<string, JsonValue> {
  const forbidden = draft.metric === 'forbidden_terms'
  const data: Record<string, JsonValue> = {
    field: draft.field.trim(),
    metric: draft.metric,
    op: forbidden ? 'not_contains' : draft.op,
    value: forbidden
      ? words(draft.terms)
      : draft.op === 'between'
        ? [draft.low ?? 0, draft.high ?? 0]
        : (draft.low ?? 0),
    scope: draft.scope,
    severity: draft.severity,
    active: draft.active,
  }
  if (draft.pattern.trim() !== '' && (draft.metric === 'match_count' || draft.metric === 'share'))
    data.pattern = draft.pattern.trim()
  const kept = words(draft.filterEquals)
  if (draft.filterField.trim() !== '' && kept.length > 0) {
    data.filter = {
      field: draft.filterField.trim(),
      equals: kept,
      ...(draft.filterNot ? { not: true } : {}),
    }
  }
  return data
}

/** Whether the boxes say enough to be written: the backend says the rest. */
export function draftReady(draft: QuotaDraft): boolean {
  if (draft.field.trim() === '') return false
  if (draft.metric === 'forbidden_terms') return words(draft.terms).length > 0
  if (draft.metric === 'match_count' && draft.pattern.trim() === '') return false
  if (draft.metric === 'share' && draft.scope === 'item') return false
  if (draft.low === null) return false
  return draft.op !== 'between' || (draft.high !== null && draft.high >= draft.low)
}

/** What the rule wants, as the check says it: `400-500`, `<= 4`. */
export function expectedOf(draft: QuotaDraft): string {
  if (draft.metric === 'forbidden_terms') return words(draft.terms).join(', ')
  const low = String(draft.low ?? 0)
  switch (draft.op) {
    case 'between':
      return `${low}-${String(draft.high ?? 0)}`
    case 'min':
      return `>= ${low}`
    case 'max':
      return `<= ${low}`
    case 'eq':
      return `= ${low}`
  }
}

/**
 * The fields a rule can name in this profile, for the box that takes one:
 * every work's own, each kind's roles and overview fields, and its doors'
 * fields - plain and by door. Offered, not required: the backend reads what
 * a work's kind has and passes a work by that has no such field.
 */
export function quotaFields(config: ProfileConfig): string[] {
  const found = new Set<string>(['title', 'kind', 'status', 'stage', 'tags'])
  for (const kind of config.work_kinds) {
    if (kind.cover === true) {
      found.add('cover')
      found.add('cover_scene')
      found.add('cover_prompt')
    }
    for (const role of kind.version_roles ?? []) found.add(role.key)
    for (const door of kind.release_kinds ?? []) {
      for (const field of door.fields ?? []) {
        found.add(field.key)
        found.add(`${door.key}.${field.key}`)
      }
    }
  }
  for (const field of config.work_meta_fields) found.add(field.key)
  return [...found]
}
