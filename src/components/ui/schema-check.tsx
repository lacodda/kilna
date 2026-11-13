import { patternOf, type SchemaField, type SchemaRules } from './schema-shape'

/*
 * What is wrong with a value, by the rules its schema states.
 *
 * The other half of `schema-shape`: that file reads which fields there are,
 * this one holds a value against them. It answers in codes - `minimum`, with
 * the limit - and never in words, because the words are the product's and in
 * the product's language; `SchemaFields` asks the product to say each one.
 *
 * It checks what the form draws and nothing more. A format it does not know
 * (`hostname`, `uuid`) is drawn as text and not checked here, and a rule the
 * schema cannot state - a zod `refine`, a name that is already taken - arrives
 * as an error from outside. This is the check that lets a form refuse a value
 * before it is sent, not a replacement for the one the receiving end makes.
 */

type Limit =
  | 'minLength'
  | 'maxLength'
  | 'minimum'
  | 'maximum'
  | 'exclusiveMinimum'
  | 'exclusiveMaximum'
  | 'multipleOf'
  | 'minItems'
  | 'maxItems'

/** One problem with one field. `key` is the field's; `item` is set when the
 * problem is one entry of a list of tags. */
export type SchemaIssue = { key: string; item?: string } & (
  | { code: 'required' | 'type' | 'enum' | 'uniqueItems' }
  | { code: 'format'; format: string }
  | { code: 'pattern'; pattern: string }
  | { code: Limit; limit: number }
)

/** The value at a path, or `undefined` if any step of the way is missing. */
export function valueAt(value: unknown, path: readonly string[]): unknown {
  let at = value
  for (const name of path) {
    if (typeof at !== 'object' || at === null) return undefined
    at = (at as Record<string, unknown>)[name]
  }
  return at
}

/** A copy of the object with the value at the path replaced - or the key
 * removed, for `undefined`. The objects on the way are copied, never
 * changed, so a product's state stays a value React can compare. */
export function withValue(value: Record<string, unknown>, path: readonly string[], next: unknown) {
  const [name, ...rest] = path
  const copy = { ...value }
  if (name === undefined) return copy
  if (rest.length > 0) {
    const inner = copy[name]
    copy[name] = withValue(typeof inner === 'object' && inner !== null ? (inner as Record<string, unknown>) : {}, rest, next)
  } else if (next === undefined) {
    delete copy[name]
  } else {
    copy[name] = next
  }
  return copy
}

/** Every problem with the value, field by field, in the order they are drawn;
 * the first for a field is the one worth saying. */
export function checkValue(fields: readonly SchemaField[], value: unknown): SchemaIssue[] {
  return fields.flatMap((field) => checkField(field, valueAt(value, field.path), value))
}

function checkField(field: SchemaField, value: unknown, root: unknown): SchemaIssue[] {
  const { key } = field
  if (field.control === 'custom' || field.fixed) return []
  // A group is checked through its fields, present or not: a required group
  // is there by construction (`defaultsOf` makes it), and an optional one
  // that is absent asks nothing of what would be inside it.
  if (field.control === 'group') {
    if (value === undefined && !field.required) return []
    return checkValue(field.fields ?? [], root)
  }
  if (value === undefined || value === '' || (value === null && !field.nullable)) {
    return field.required ? [{ key, code: 'required' }] : []
  }
  if (value === null) return []

  switch (field.control) {
    case 'number':
    case 'integer':
      if (typeof value !== 'number' || !Number.isFinite(value)) return [{ key, code: 'type' }]
      if (field.control === 'integer' && !Number.isInteger(value)) return [{ key, code: 'type' }]
      return numberIssues(key, value, field.rules)
    case 'boolean':
      return typeof value === 'boolean' ? [] : [{ key, code: 'type' }]
    case 'choice':
    case 'segmented':
    case 'radio':
      return field.options?.some((option) => option.value === value) ? [] : [{ key, code: 'enum' }]
    case 'choices':
    case 'tags': {
      if (!Array.isArray(value)) return [{ key, code: 'type' }]
      const issues = listIssues(key, value, field.rules)
      for (const item of value) {
        if (field.control === 'choices') {
          if (!field.options?.some((option) => option.value === item)) issues.push({ key, code: 'enum' })
        } else if (typeof item !== 'string') {
          issues.push({ key, code: 'type' })
        } else {
          issues.push(...textIssues(key, item, field.itemRules ?? {}).map((issue) => ({ ...issue, item })))
        }
      }
      return issues
    }
    default:
      return typeof value === 'string' ? textIssues(key, value, field.rules) : [{ key, code: 'type' }]
  }
}

/** Formats a reader can be told about. Deliberately loose: an address with an
 * `@` and no spaces is an address to try, and the mail server is the judge of
 * the rest - a strict pattern refuses real addresses, which is worse. */
const FORMATS: Record<string, (value: string) => boolean> = {
  email: (value) => /^[^\s@]+@[^\s@]+$/.test(value),
  uri: (value) => URL.canParse(value),
  date: (value) => {
    const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value)
    if (!match) return false
    const [year, month, day] = match.slice(1).map(Number) as [number, number, number]
    const date = new Date(Date.UTC(year, month - 1, day))
    return date.getUTCMonth() === month - 1 && date.getUTCDate() === day
  },
}

function textIssues(key: string, value: string, rules: SchemaRules): SchemaIssue[] {
  const issues: SchemaIssue[] = []
  // Characters as a reader counts them, which is what JSON Schema counts: an
  // emoji is one, not the two UTF-16 units `length` sees.
  const length = [...value].length
  if (rules.minLength !== undefined && length < rules.minLength) issues.push({ key, code: 'minLength', limit: rules.minLength })
  if (rules.maxLength !== undefined && length > rules.maxLength) issues.push({ key, code: 'maxLength', limit: rules.maxLength })
  const known = rules.format === undefined ? undefined : FORMATS[rules.format]
  if (known && !known(value)) issues.push({ key, code: 'format', format: rules.format! })
  if (rules.pattern !== undefined && !patternOf(rules.pattern, key).test(value)) {
    issues.push({ key, code: 'pattern', pattern: rules.pattern })
  }
  return issues
}

function numberIssues(key: string, value: number, rules: SchemaRules): SchemaIssue[] {
  const issues: SchemaIssue[] = []
  if (rules.minimum !== undefined && value < rules.minimum) issues.push({ key, code: 'minimum', limit: rules.minimum })
  if (rules.maximum !== undefined && value > rules.maximum) issues.push({ key, code: 'maximum', limit: rules.maximum })
  if (rules.exclusiveMinimum !== undefined && value <= rules.exclusiveMinimum) {
    issues.push({ key, code: 'exclusiveMinimum', limit: rules.exclusiveMinimum })
  }
  if (rules.exclusiveMaximum !== undefined && value >= rules.exclusiveMaximum) {
    issues.push({ key, code: 'exclusiveMaximum', limit: rules.exclusiveMaximum })
  }
  // A quotient within a hair of a whole number is a whole number: 0.3 / 0.1 is
  // 2.9999999999999996 in floating point, and 0.3 is a multiple of 0.1.
  if (rules.multipleOf !== undefined) {
    const quotient = value / rules.multipleOf
    if (Math.abs(quotient - Math.round(quotient)) > 1e-9) issues.push({ key, code: 'multipleOf', limit: rules.multipleOf })
  }
  return issues
}

function listIssues(key: string, value: readonly unknown[], rules: SchemaRules): SchemaIssue[] {
  const issues: SchemaIssue[] = []
  if (rules.minItems !== undefined && value.length < rules.minItems) issues.push({ key, code: 'minItems', limit: rules.minItems })
  if (rules.maxItems !== undefined && value.length > rules.maxItems) issues.push({ key, code: 'maxItems', limit: rules.maxItems })
  if (rules.uniqueItems && new Set(value).size < value.length) issues.push({ key, code: 'uniqueItems' })
  return issues
}
