import { useMemo, useState, type ReactNode } from 'react'
import { cn } from 'dowel-ui'
import { Field, FieldGroup } from './field'
import { Input } from './input'
import { Textarea } from './textarea'
import { PasswordField } from './password-field'
import { NumberField } from './number-field'
import { Switch } from './switch'
import { Select, SelectItem, SelectPopup, SelectTrigger, SelectValue } from './select'
import { SegmentedControl, Segment } from './segmented-control'
import { Radio, RadioGroup } from './radio-group'
import { Checkbox, CheckboxGroup } from './checkbox'
import { TagInput } from './tag-input'
import { DatePicker } from './date-picker'
import { readSchema, type JsonSchema, type SchemaField, type SchemaOption } from './schema-shape'
import { checkValue, valueAt, withValue, type SchemaIssue } from './schema-check'

/*
 * SchemaFields - the fields a JSON Schema describes, drawn and checked.
 *
 * A settings screen, a profile, a config: the data is already described once,
 * as a zod schema or a Rust struct, and the form beside it was the second
 * description - the one that fell behind. This draws the form from the first
 * one. zod hands it a schema with `z.toJSONSchema()`, a Rust struct with
 * `schemars`; no adapter for either, because the schema is the adapter. Which
 * field becomes which control is `schema-shape`'s reading, and what is wrong
 * with a value is `schema-check`'s.
 *
 * Every field is a `Field` or a `FieldGroup`, so the label, the hint and the
 * error are wired the one way the set wires them. The value is the product's:
 * one object in, the next object out, nothing kept here but which fields have
 * been left - a problem is said once a field has been left, or for all of them
 * at once after a refused submit (`reveal`). An empty box writes nothing: the
 * key is removed, or set to `null` where the schema allows `null`, so an
 * optional field left empty is absent rather than an empty string that fails
 * its own pattern.
 *
 * It has no words of its own. What a problem is called, what the empty choice
 * says, what the date picker's buttons are named - all `words`, in the
 * product's language. A field the schema cannot describe well enough is the
 * product's to draw (`custom`); one it cannot describe at all is refused by
 * name when the schema is read, never silently left out.
 */

/** Every word the fields can show, in the product's language. */
export interface SchemaFieldsWords {
  /** What is wrong, said for one field: `issue.code` names the rule, `limit`
   * and `format` carry what it asked for. */
  issue: (issue: SchemaIssue, field: SchemaField) => ReactNode
  /** The answer that takes an optional choice back: "None". */
  none: string
  /** What a required choice says before anything is chosen: "Choose…". */
  choose: string
  /** The date picker's empty face and its two month buttons. */
  pickDate: string
  previousMonth: string
  nextMonth: string
  /** What takes back a date in an optional field: "Clear". */
  clearDate: string
  /** The secret's reveal button, while hidden and while showing. */
  showSecret: string
  hideSecret: string
  /** Names the cross on one tag: "Remove {tag}". */
  removeTag: (tag: string) => string
}

/** What a product's own drawing of a field is handed. */
export interface SchemaCustomProps {
  field: SchemaField
  value: unknown
  onValueChange: (value: unknown) => void
  error: ReactNode
  disabled: boolean
}

export interface SchemaFieldsProps {
  /** An object schema: its properties are the fields, in the order written. */
  schema: JsonSchema
  /** The record the fields edit, by property key. */
  value: Readonly<Record<string, unknown>>
  /** Told the next record - a copy, never the same object changed. */
  onValueChange: (value: Record<string, unknown>) => void
  /** Every word the fields show, so the product can translate them. */
  words: SchemaFieldsWords
  /** Problems from elsewhere - the server, a rule the schema cannot state - by
   * key (`server.port`). Said at once, and over the schema's own. */
  errors?: Readonly<Record<string, ReactNode>>
  /** Say every problem now, left or not - after a submit that was refused. */
  reveal?: boolean
  /** Fields the product draws itself, by key. Read for their label and help;
   * never refused, never checked here. */
  custom?: Readonly<Record<string, (props: SchemaCustomProps) => ReactNode>>
  /** Disables every field at once, for example while the record saves. */
  disabled?: boolean
  className?: string
}

/** An option's value as the string a control carries. `JSON.stringify` keeps
 * `1` and `'1'` apart, and never yields the empty string the "none" answer
 * takes. */
const keyOf = (value: unknown) => JSON.stringify(value)

export function SchemaFields({
  schema,
  value,
  onValueChange,
  words,
  errors,
  reveal = false,
  custom,
  disabled = false,
  className,
}: SchemaFieldsProps) {
  // The keys of `custom` rather than the object: they are what reading
  // depends on, and an inline object is a new identity on every render.
  const drawn = Object.keys(custom ?? {}).join('\n')
  const fields = useMemo(() => readSchema(schema, drawn ? drawn.split('\n') : []), [schema, drawn])
  const [left, setLeft] = useState<ReadonlySet<string>>(new Set())

  const issues = useMemo(() => {
    const first = new Map<string, SchemaIssue>()
    for (const issue of checkValue(fields, value)) if (!first.has(issue.key)) first.set(issue.key, issue)
    return first
  }, [fields, value])

  const draw = (field: SchemaField): ReactNode => {
    if (field.fixed) return null
    if (field.control === 'group') {
      return (
        <FieldGroup key={field.key} label={field.label} help={field.help} required={field.required} disabled={disabled}>
          <div className="flex flex-col gap-4">{field.fields?.map(draw)}</div>
        </FieldGroup>
      )
    }
    const issue = issues.get(field.key)
    const error =
      errors?.[field.key] ?? (issue && (reveal || left.has(field.key)) ? words.issue(issue, field) : undefined)
    const set = (next: unknown) => onValueChange(withValue(value, field.path, next))
    return (
      <div
        key={field.key}
        // Left means focus went somewhere outside the field, not from its box
        // to its own reveal button.
        onBlur={(event) => {
          if (event.currentTarget.contains(event.relatedTarget as Node | null)) return
          setLeft((keys) => (keys.has(field.key) ? keys : new Set(keys).add(field.key)))
        }}
      >
        {control(field, valueAt(value, field.path), set, error)}
      </div>
    )
  }

  const control = (field: SchemaField, current: unknown, set: (next: unknown) => void, error: ReactNode) => {
    const empty = field.nullable ? null : undefined
    const group = { label: field.label, help: field.help, required: field.required, error, disabled }
    const shared = { ...group, name: field.key }
    const text = typeof current === 'string' ? current : ''
    const options = field.options ?? []
    const optionOf = (key: string) => options.find((option) => keyOf(option.value) === key)
    const choose = (key: string | null) => set(key ? optionOf(key)?.value : empty)

    switch (field.control) {
      case 'custom':
        return custom?.[field.key]?.({ field, value: current, onValueChange: set, error, disabled })
      case 'text':
      case 'email':
      case 'url':
        return (
          <Field {...shared}>
            <Input
              type={field.control === 'text' ? 'text' : field.control}
              value={text}
              readOnly={field.readOnly}
              required={field.required}
              onChange={(event) => set(event.target.value === '' ? empty : event.target.value)}
            />
          </Field>
        )
      case 'multiline':
        return (
          <Field {...shared}>
            <Textarea
              autoResize
              value={text}
              readOnly={field.readOnly}
              required={field.required}
              onChange={(event) => set(event.target.value === '' ? empty : event.target.value)}
            />
          </Field>
        )
      case 'secret':
        return (
          <Field {...shared}>
            <PasswordField
              value={text}
              // Not a sign-in: a token or a key in a settings screen, which a
              // password manager should neither fill nor offer to generate.
              autoComplete="off"
              showLabel={words.showSecret}
              hideLabel={words.hideSecret}
              readOnly={field.readOnly}
              required={field.required}
              onValueChange={(next) => set(next === '' ? empty : next)}
            />
          </Field>
        )
      case 'date':
        return (
          <Field {...shared}>
            <DatePicker
              value={text || undefined}
              placeholder={words.pickDate}
              previousMonthLabel={words.previousMonth}
              nextMonthLabel={words.nextMonth}
              disabled={disabled || field.readOnly}
              // An optional date can be taken back, and is then absent - or
              // null, where the schema allows it - like every empty field.
              {...(field.required ? {} : { clearLabel: words.clearDate })}
              onValueChange={(next: string | undefined) => set(next === undefined ? empty : next)}
            />
          </Field>
        )
      case 'number':
      case 'integer': {
        const { minimum, maximum, multipleOf } = field.rules
        return (
          <Field {...shared}>
            <NumberField
              value={typeof current === 'number' ? current : null}
              disabled={disabled}
              min={minimum}
              max={maximum}
              // A number out of range is said, not quietly moved to the bound.
              allowOutOfRange
              step={multipleOf ?? (field.control === 'integer' ? 1 : undefined)}
              format={field.control === 'integer' ? { maximumFractionDigits: 0 } : undefined}
              readOnly={field.readOnly}
              required={field.required}
              onValueChange={(next) => set(next === null ? empty : next)}
            />
          </Field>
        )
      }
      case 'boolean':
        return (
          <Field {...shared} beside>
            <Switch
              disabled={disabled}
              // Absent reads as the default: that is what the receiving end
              // will take it to be.
              checked={typeof current === 'boolean' ? current : field.default === true}
              readOnly={field.readOnly}
              onCheckedChange={set}
            />
          </Field>
        )
      case 'choice': {
        const items = [
          ...(field.required ? [] : [{ value: '', label: words.none }]),
          ...options.map((option) => ({ value: keyOf(option.value), label: option.label })),
        ]
        const chosen = current === undefined || current === null ? (field.required ? null : '') : keyOf(current)
        return (
          <Field {...shared}>
            <Select
              items={items}
              value={chosen}
              disabled={disabled}
              readOnly={field.readOnly}
              required={field.required}
              onValueChange={(next: string | null) => choose(next)}
            >
              <SelectTrigger>
                <SelectValue placeholder={words.choose} />
              </SelectTrigger>
              <SelectPopup>
                {items.map((item) => (
                  <SelectItem key={item.value} value={item.value}>
                    {item.label}
                  </SelectItem>
                ))}
              </SelectPopup>
            </Select>
          </Field>
        )
      }
      case 'segmented':
        return (
          <FieldGroup {...group}>
            <SegmentedControl
              aria-label={field.label}
              value={current === undefined || current === null ? '' : keyOf(current)}
              disabled={disabled || field.readOnly}
              onValueChange={choose}
            >
              {options.map((option) => (
                <Segment key={keyOf(option.value)} value={keyOf(option.value)}>
                  {option.label}
                </Segment>
              ))}
            </SegmentedControl>
          </FieldGroup>
        )
      case 'radio':
        return (
          <FieldGroup {...group}>
            <RadioGroup
              aria-label={field.label}
              value={current === undefined || current === null ? '' : keyOf(current)}
              readOnly={field.readOnly}
              required={field.required}
              onValueChange={choose}
            >
              {options.map((option) => (
                <Radio key={keyOf(option.value)} value={keyOf(option.value)}>
                  <Answer option={option} />
                </Radio>
              ))}
            </RadioGroup>
          </FieldGroup>
        )
      case 'choices': {
        const picked = Array.isArray(current) ? current.map(keyOf) : []
        return (
          <FieldGroup {...group}>
            <CheckboxGroup
              value={picked}
              disabled={disabled || field.readOnly}
              // In the order the schema offers them, whatever order they were
              // ticked in: a stored list that reorders itself on every edit is
              // a diff nobody asked for.
              onValueChange={(next) =>
                set(options.filter((option) => next.includes(keyOf(option.value))).map((option) => option.value))
              }
            >
              {options.map((option) => (
                <Checkbox key={keyOf(option.value)} value={keyOf(option.value)}>
                  <Answer option={option} />
                </Checkbox>
              ))}
            </CheckboxGroup>
          </FieldGroup>
        )
      }
      case 'tags':
        return (
          <Field {...shared}>
            <TagInput
              value={Array.isArray(current) ? current.filter((tag) => typeof tag === 'string') : []}
              max={field.rules.maxItems}
              removeLabel={words.removeTag}
              disabled={disabled || field.readOnly}
              onValueChange={set}
            />
          </Field>
        )
    }
  }

  return <div className={cn('flex flex-col gap-4', className)}>{fields.map(draw)}</div>
}

/** An answer, with its own line of help where the schema gives one - a radio
 * or a checkbox has room for it, a select does not. */
function Answer({ option }: { option: SchemaOption }) {
  if (option.help === undefined) return option.label
  return (
    <span className="flex flex-col">
      <span>{option.label}</span>
      <span className="text-xs text-dim">{option.help}</span>
    </span>
  )
}
