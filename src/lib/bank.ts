import type { JsonValue } from '@/lib/api/generated/serde_json/JsonValue'
import type { BankState, BankUse } from '@/lib/api/types'

/*
 * An item of a bank as the window reads it (v0.151, FR-20 20.2): where it
 * stands today, and the boxes its editor fills. The rule of a rest is the
 * backend's (`canon::bank::state`), said again here for the day the screen
 * is looked at rather than the day the prompt was written.
 */

const text = (value: JsonValue | undefined) => (typeof value === 'string' ? value : '')

/** Where an item stands on `today` (`YYYY-MM-DD`), and - resting - the month
 *  it is free again. */
export function bankState(
  data: Record<string, JsonValue | undefined>,
  today: string,
): { state: BankState; until: string | null } {
  if (data.locked === true) return { state: 'locked', until: null }
  const months = typeof data.cooldown_months === 'number' ? data.cooldown_months : 0
  const last = text(data.last_used_at)
  if (months <= 0 || !/^\d{4}-\d{2}-\d{2}$/.test(last)) return { state: 'available', until: null }
  const free = plusMonths(last, months)
  return free > today
    ? { state: 'cooldown', until: free.slice(0, 7) }
    : { state: 'available', until: null }
}

/** `day` and `months` later, on the same day or the month's last. */
export function plusMonths(day: string, months: number): string {
  const [year, month, date] = day.split('-').map(Number) as [number, number, number]
  const index = year * 12 + (month - 1) + months
  const nextYear = Math.floor(index / 12)
  const nextMonth = (index % 12) + 1
  const last = new Date(Date.UTC(nextYear, nextMonth, 0)).getUTCDate()
  const pad = (n: number, width: number) => String(n).padStart(width, '0')
  return `${pad(nextYear, 4)}-${pad(nextMonth, 2)}-${pad(Math.min(date, last), 2)}`
}

/** The works that used an item, earliest first. */
export function usesOf(data: Record<string, JsonValue | undefined>): BankUse[] {
  const uses = data.used_in
  if (!Array.isArray(uses)) return []
  return uses.flatMap((one) =>
    one !== null &&
    typeof one === 'object' &&
    !Array.isArray(one) &&
    typeof one.work_id === 'string' &&
    typeof one.date === 'string'
      ? [{ work_id: one.work_id, title: text(one.title), date: one.date }]
      : [],
  )
}

export interface BankDraft {
  group: string
  /** Comma by comma. */
  attributes: string
  cooldown: number | null
  lastUsed: string
  locked: boolean
  note: string
}

export function bankDraftOf(data: Record<string, JsonValue | undefined>): BankDraft {
  const attributes = data.attributes
  return {
    group: text(data.group),
    attributes: Array.isArray(attributes)
      ? attributes.filter((one): one is string => typeof one === 'string').join(', ')
      : text(attributes),
    cooldown: typeof data.cooldown_months === 'number' ? data.cooldown_months : null,
    lastUsed: text(data.last_used_at),
    locked: data.locked === true,
    note: text(data.note),
  }
}

/** The data an item is written with: its boxes, and the uses it already
 *  holds - those are marked from a work, not typed. */
export function bankDataOf(
  draft: BankDraft,
  before: Record<string, JsonValue | undefined>,
): Record<string, JsonValue> {
  const data: Record<string, JsonValue> = {}
  if (draft.group.trim() !== '') data.group = draft.group.trim()
  const attributes = draft.attributes
    .split(',')
    .map((one) => one.trim())
    .filter((one) => one !== '')
  if (attributes.length > 0) data.attributes = attributes
  if (draft.cooldown !== null) data.cooldown_months = Math.max(0, Math.trunc(draft.cooldown))
  if (draft.lastUsed !== '') data.last_used_at = draft.lastUsed
  const uses = before.used_in
  if (Array.isArray(uses) && uses.length > 0) data.used_in = uses
  if (draft.locked) data.locked = true
  if (draft.note.trim() !== '') data.note = draft.note.trim()
  return data
}
