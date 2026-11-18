import type { Label, ProfileConfig } from '@/lib/api/types'
import { say } from '@/lib/useProfile'

/**
 * Reading an offer of what kilna ships now (ADR 0087): which list it is in,
 * in the profile's words, and which fields of the entry differ.
 */

/** One field of an entry that reads otherwise in the space and in the build. */
export interface FieldChange {
  field: string
  mine: string
  standard: string
  /** Text of several lines, compared line by line rather than side by side. */
  long: boolean
}

/** A value as it reads in one line: a word in the window's language, a list
 *  of words joined, anything else as the JSON it is. */
export function shown(value: unknown): string {
  if (value === undefined || value === null) return ''
  if (typeof value === 'string') return value
  if (typeof value === 'number' || typeof value === 'boolean') return String(value)
  if (isLabel(value)) return Object.values(value).join(' / ')
  if (Array.isArray(value) && value.every((one) => typeof one === 'string')) {
    return value.join(', ')
  }
  return JSON.stringify(value)
}

function isLabel(value: unknown): value is Record<string, string> {
  return (
    typeof value === 'object' &&
    value !== null &&
    !Array.isArray(value) &&
    Object.values(value).every((one) => typeof one === 'string')
  )
}

/** The fields that differ, in the order the shipped entry names them, then
 *  any only the space's entry has. */
export function changes(
  mine: Record<string, unknown>,
  standard: Record<string, unknown>,
): FieldChange[] {
  const fields = [
    ...Object.keys(standard),
    ...Object.keys(mine).filter((key) => !(key in standard)),
  ]
  return fields
    .filter((field) => JSON.stringify(mine[field]) !== JSON.stringify(standard[field]))
    .map((field) => {
      const a = shown(mine[field])
      const b = shown(standard[field])
      return { field, mine: a, standard: b, long: a.includes('\n') || b.includes('\n') }
    })
}

/** What the entry is called: its label in the window's language, or its key. */
export function entryName(entry: Record<string, unknown>, key: string): string {
  const label = entry.label
  return typeof label === 'string' || isLabel(label) ? say(label as Label) : key
}

/** The list an offer is in, as `[word, of]`: the word names what the entry
 *  is - `settings.shipped.list.statuses` - and `of` the kind it belongs to,
 *  for the lists inside a kind. */
export function listOf(collection: string, config: ProfileConfig): { word: string; of?: string } {
  const parts = collection.split('/')
  if (parts[0] === 'work_kinds' && parts.length === 3) {
    const kind = config.work_kinds.find((one) => one.key === parts[1])
    return { word: parts[2]!, of: kind === undefined ? parts[1] : say(kind.label) }
  }
  if (parts[0] === 'note_kinds' && parts.length === 3) {
    const kind = config.note_kinds.find((one) => one.key === parts[1])
    return { word: parts[2]!, of: kind === undefined ? parts[1] : say(kind.label) }
  }
  return { word: parts.join('_') }
}
