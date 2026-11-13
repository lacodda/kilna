export {}
// Code first, so the description below survives `shadcn add`: the CLI writes
// a file from its first token on and drops every comment above it.

/*
 * A JSON Schema, read as the fields a form draws - with no React in it.
 *
 * Settings, a profile, a config file: the description of the data already
 * exists, as a zod schema or a Rust struct, and the form was being written a
 * second time beside it by hand. The two drift - a field added to the struct
 * that nobody can edit, a range the form allows and the server refuses.
 * JSON Schema is what both ends already speak: zod writes one with
 * `z.toJSONSchema()`, a Rust struct with `schemars`. So that is what is read
 * here, and no adapter for either library is needed - the schema is the
 * adapter.
 *
 * Reading is strict on purpose. A keyword this cannot draw - a union of
 * objects, a map, a list of records - is refused by name rather than skipped,
 * because a form that silently leaves a field out is a form the data can no
 * longer be edited through, and nobody notices until the value is wrong. The
 * product draws such a field itself (`custom`), or changes the schema.
 *
 * Only the dialects the two writers produce are read: draft 2020-12 and
 * draft 7, local `$ref` into `$defs` or `definitions`, nullable spelt either
 * as a type list or as a union with `null`, and a choice spelt as `enum` or
 * as a union of `const`s carrying a `title`.
 */

/** A JSON Schema, or one of its parts. `true` and `false` are schemas too. */
export type JsonSchema = boolean | { readonly [keyword: string]: unknown }

/** How a field is drawn. The data's type picks it; `x-control` in the schema
 * picks between the drawings one type has. */
export type SchemaControl =
  | 'text'
  | 'multiline'
  | 'secret'
  | 'email'
  | 'url'
  | 'date'
  | 'number'
  | 'integer'
  | 'boolean'
  | 'choice'
  | 'segmented'
  | 'radio'
  | 'choices'
  | 'tags'
  | 'group'
  /** The product draws it - named in `custom` when the schema was read. */
  | 'custom'

/** One answer a choice offers. */
export interface SchemaOption {
  value: string | number | boolean
  /** `title` if the schema gives one, else the value as written. */
  label: string
  help?: string
}

/** The keywords a value is checked against, as the schema states them. */
export interface SchemaRules {
  minLength?: number
  maxLength?: number
  pattern?: string
  format?: string
  minimum?: number
  maximum?: number
  exclusiveMinimum?: number
  exclusiveMaximum?: number
  multipleOf?: number
  minItems?: number
  maxItems?: number
  uniqueItems?: boolean
}

export interface SchemaField {
  /** Where the value lives in the object: `['server', 'port']`. */
  path: readonly string[]
  /** The path as one string, `server.port` - what `errors` and `custom` are
   * keyed by. */
  key: string
  control: SchemaControl
  /** `title`, else the property's own name. Never a word of this file's. */
  label: string
  help?: string
  required: boolean
  /** Empty is written as `null` rather than by leaving the key out. */
  nullable: boolean
  readOnly: boolean
  default?: unknown
  /** A value the schema allows and no person chooses - a discriminator. Not
   * drawn; `defaultsOf` writes it. */
  fixed?: { value: unknown }
  rules: SchemaRules
  /** The rules each entry of a list of tags is held to. */
  itemRules?: SchemaRules
  options?: readonly SchemaOption[]
  fields?: readonly SchemaField[]
}

/** A schema this cannot draw, by the key of the field and the reason. */
export class SchemaError extends Error {
  readonly key: string
  readonly reason: string

  constructor(key: string, reason: string) {
    super(`${key || 'the schema'}: ${reason}`)
    this.name = 'SchemaError'
    this.key = key
    this.reason = reason
  }
}

type Node = { readonly [keyword: string]: unknown }

const RULES = [
  'minLength',
  'maxLength',
  'pattern',
  'format',
  'minimum',
  'maximum',
  'exclusiveMinimum',
  'exclusiveMaximum',
  'multipleOf',
  'minItems',
  'maxItems',
  'uniqueItems',
] as const

/** The keywords that make a schema something this file cannot draw. Each is
 * refused by name, so the message says what to change. */
const REFUSED = ['patternProperties', 'prefixItems', 'not', 'if', 'dependentSchemas']

/** Where a drawing chosen by `x-control` is allowed: the type it redraws. */
const REDRAWS: Record<string, SchemaControl> = { multiline: 'text', segmented: 'choice', radio: 'choice' }

const isNode = (value: unknown): value is Node =>
  typeof value === 'object' && value !== null && !Array.isArray(value)

const isNull = (node: unknown) => isNode(node) && (node.type === 'null' || node.const === null)

/** The node with one keyword taken out - what stands beside a `$ref` or a
 * union, kept so it can be laid over what the keyword resolves to. */
function without(node: Node, keyword: string): Node {
  const rest: Record<string, unknown> = { ...node }
  delete rest[keyword]
  return rest
}

/**
 * The fields an object schema describes, in the order its properties are
 * written - which is the order zod and serde declare them in.
 *
 * `custom` names the keys the product draws itself: they are read for their
 * label and help, and never refused.
 */
export function readSchema(schema: JsonSchema, custom: Iterable<string> = []): SchemaField[] {
  if (!isNode(schema)) throw new SchemaError('', 'the root is not an object schema')
  const read = new Reader(schema, new Set(custom))
  const root = read.resolve(schema, '', [])
  if (!isNode(root.properties)) throw new SchemaError('', 'the root has no properties')
  return read.fields(root, [])
}

class Reader {
  private readonly root: Node
  private readonly custom: ReadonlySet<string>

  constructor(root: Node, custom: ReadonlySet<string>) {
    this.root = root
    this.custom = custom
  }

  /** `$ref`, a lone `allOf` and nullability peeled off, so what is left says
   * what the value is. Keywords beside a `$ref` win over the target's - in
   * 2020-12 a description at the use site describes the use. */
  resolve(node: Node, key: string, seen: readonly string[]): Node & { nullable?: boolean } {
    if (typeof node.$ref === 'string') {
      const ref = node.$ref
      if (seen.includes(ref)) throw new SchemaError(key, `\`${ref}\` refers to itself, and a form cannot be drawn to no end`)
      const beside = without(node, '$ref')
      return this.resolve({ ...this.pointer(ref, key), ...beside }, key, [...seen, ref])
    }
    if (Array.isArray(node.allOf)) {
      if (node.allOf.length !== 1 || !isNode(node.allOf[0])) throw new SchemaError(key, '`allOf` of several schemas is not read')
      const beside = without(node, 'allOf')
      return this.resolve({ ...node.allOf[0], ...beside }, key, seen)
    }
    for (const union of ['anyOf', 'oneOf'] as const) {
      const members = node[union]
      if (!Array.isArray(members) || !members.some(isNull)) continue
      // `null` among the members is the field being allowed to be empty, not
      // one more thing it can be: zod writes `.nullable()` this way, and
      // schemars writes `Option<T>` this way when T is a reference.
      const rest = members.filter((member) => !isNull(member))
      const beside = without(node, union)
      const inner = rest.length === 1 && isNode(rest[0]) ? { ...rest[0], ...beside } : { ...beside, [union]: rest }
      return { ...this.resolve(inner, key, seen), nullable: true }
    }
    if (Array.isArray(node.type)) {
      const types = node.type.filter((type) => type !== 'null')
      if (types.length !== 1) throw new SchemaError(key, `a value of several types (${node.type.join(', ')}) is not read`)
      return { ...node, type: types[0], nullable: node.nullable === true || types.length < node.type.length }
    }
    if (Array.isArray(node.enum) && node.enum.includes(null)) return { ...node, nullable: true }
    return node
  }

  /** A local JSON pointer: `#/$defs/Server`. A remote one would need a fetch
   * this file is in no position to make. */
  pointer(ref: string, key: string): Node {
    if (!ref.startsWith('#/')) throw new SchemaError(key, `\`${ref}\` is not a local reference`)
    let at: unknown = this.root
    for (const part of ref.slice(2).split('/')) {
      const name = decodeURIComponent(part).replace(/~1/g, '/').replace(/~0/g, '~')
      at = isNode(at) ? at[name] : undefined
    }
    if (!isNode(at)) throw new SchemaError(key, `\`${ref}\` points at nothing`)
    return at
  }

  fields(node: Node, path: readonly string[]): SchemaField[] {
    const required = Array.isArray(node.required) ? node.required : []
    return Object.entries(node.properties as Node).map(([name, child]) => {
      const at = [...path, name]
      return this.field(isNode(child) ? child : {}, at, required.includes(name))
    })
  }

  field(raw: Node, path: readonly string[], required: boolean): SchemaField {
    const key = path.join('.')
    const node = this.resolve(raw, key, [])
    const field: SchemaField = {
      path,
      key,
      control: 'custom',
      label: typeof node.title === 'string' ? node.title : path[path.length - 1]!,
      help: typeof node.description === 'string' ? node.description : undefined,
      required,
      nullable: node.nullable === true,
      readOnly: node.readOnly === true,
      default: node.default,
      rules: rulesOf(node, key),
    }
    if (this.custom.has(key)) return field
    if ('const' in node) return { ...field, fixed: { value: node.const } }
    const refused = REFUSED.find((keyword) => keyword in node)
    if (refused) throw new SchemaError(key, `\`${refused}\` is not read; draw the field with \`custom\``)

    const control = this.control(node, field)
    const redraw = node['x-control']
    if (redraw === undefined) return { ...field, ...control }
    if (typeof redraw !== 'string' || REDRAWS[redraw] !== control.control) {
      throw new SchemaError(key, `\`x-control: ${String(redraw)}\` does not redraw a field drawn as ${control.control}`)
    }
    return { ...field, ...control, control: redraw as SchemaControl }
  }

  control(node: Node, field: SchemaField): Pick<SchemaField, 'control' | 'options' | 'fields' | 'itemRules'> {
    const options = optionsOf(node, field.key)
    if (options) return { control: 'choice', options }
    switch (node.type) {
      case 'object':
        if (!isNode(node.properties)) break
        if (isNode(node.additionalProperties)) break
        return { control: 'group', fields: this.fields(node, field.path) }
      case 'array': {
        if (!isNode(node.items)) break
        const items = this.resolve(node.items, field.key, [])
        if (items.nullable) break
        const choices = optionsOf(items, field.key)
        if (choices) return { control: 'choices', options: choices }
        if (items.type === 'string' && items.format === undefined) {
          return { control: 'tags', itemRules: rulesOf(items, field.key) }
        }
        break
      }
      case 'string':
        if (node.writeOnly === true || node.format === 'password') return { control: 'secret' }
        if (node.format === 'date') return { control: 'date' }
        if (node.format === 'email') return { control: 'email' }
        if (node.format === 'uri') return { control: 'url' }
        return { control: 'text' }
      case 'number':
      case 'integer':
        return { control: node.type }
      case 'boolean':
        return { control: 'boolean' }
    }
    throw new SchemaError(field.key, `${describe(node)} is not drawn; draw the field with \`custom\``)
  }
}

/** The answers a choice offers: `enum`, or a union whose every member is a
 * `const` - the spelling that can carry a title per answer. `null` among them
 * is not an answer: it is the field being empty, and is read as nullable. */
function optionsOf(node: Node, key: string): SchemaOption[] | undefined {
  if (Array.isArray(node.enum)) {
    return node.enum.filter((value) => value !== null).map((value) => option(value, key))
  }
  const members = Array.isArray(node.oneOf) ? node.oneOf : Array.isArray(node.anyOf) ? node.anyOf : undefined
  if (!members || members.length === 0 || !members.every((member) => isNode(member) && 'const' in member)) {
    return undefined
  }
  return (members as Node[]).map((member) =>
    option(member.const, key, typeof member.title === 'string' ? member.title : undefined, member.description),
  )
}

function option(value: unknown, key: string, title?: string, help?: unknown): SchemaOption {
  if (typeof value !== 'string' && typeof value !== 'number' && typeof value !== 'boolean') {
    throw new SchemaError(key, `the answer ${JSON.stringify(value)} is not a string, a number or a boolean`)
  }
  return { value, label: title ?? String(value), help: typeof help === 'string' ? help : undefined }
}

function rulesOf(node: Node, key: string): SchemaRules {
  const rules: Record<string, unknown> = {}
  for (const name of RULES) if (node[name] !== undefined) rules[name] = node[name]
  // A pattern that does not compile is the schema's mistake, said now rather
  // than on the first keystroke.
  if (typeof rules.pattern === 'string') patternOf(rules.pattern, key)
  return rules as SchemaRules
}

/** A schema's `pattern` as a RegExp. JSON Schema patterns are ECMA-262 and
 * unanchored; `u` first, as ajv reads them, and without it for a pattern
 * written for a reader that does not. */
export function patternOf(pattern: string, key = ''): RegExp {
  for (const flags of ['u', '']) {
    try {
      return new RegExp(pattern, flags)
    } catch {
      // Tried without `u` next; a pattern that fails both is reported below.
    }
  }
  throw new SchemaError(key, `the pattern \`${pattern}\` does not compile`)
}

function describe(node: Node): string {
  if (node.type === 'array') return 'a list of anything but answers or plain text'
  if (node.type === 'object') return 'an object without properties, or a map'
  if (Array.isArray(node.oneOf) || Array.isArray(node.anyOf)) return 'a union of schemas'
  return node.type === undefined ? 'a value of no stated type' : `a value of type \`${String(node.type)}\``
}

/**
 * A value for a new record: every field's `default`, and every fixed value.
 *
 * Read once, when the record is made. Showing a default in a field whose value
 * is absent would be showing a value that is not there.
 */
export function defaultsOf(fields: readonly SchemaField[]): Record<string, unknown> {
  const value: Record<string, unknown> = {}
  for (const field of fields) {
    const name = field.path[field.path.length - 1]!
    if (field.fixed) value[name] = field.fixed.value
    else if (field.default !== undefined) value[name] = structuredClone(field.default)
    else if (field.control === 'group') {
      const inner = defaultsOf(field.fields ?? [])
      if (field.required || Object.keys(inner).length > 0) value[name] = inner
    }
  }
  return value
}
