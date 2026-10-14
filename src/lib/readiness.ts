import type { TFunction } from 'i18next'
import type { Need, ProfileConfig, Readiness } from '@/lib/api/types'
import { labelOf, vocabularyOf } from '@/lib/useProfile'

// The same gap means something different a month out and two days out. The
// bands match the journal's warning horizon: inside a week a gap is worth
// amber, inside two days it is red.
export type Urgency = 'calm' | 'soon' | 'urgent'

const URGENT_WITHIN_DAYS = 2
const SOON_WITHIN_DAYS = 7

/** How loudly a gap should be shown, given how close the date is. */
export function urgency(daysLeft: number | null): Urgency {
  if (daysLeft === null || daysLeft > SOON_WITHIN_DAYS) return 'calm'
  if (daysLeft > URGENT_WITHIN_DAYS) return 'soon'
  return 'urgent'
}

/**
 * Whole days from one local ISO date to another, negative when `to` is past.
 *
 * Both dates go through `Date.UTC` from their parts: `new Date('2026-09-01')`
 * parses as UTC midnight and shifts a day in negative-offset timezones, which
 * is exactly the bug the calendar had in v0.23.
 */
export function daysBetween(from: string, to: string): number {
  return Math.round((utcMidnight(to) - utcMidnight(from)) / 86_400_000)
}

function utcMidnight(date: string): number {
  return Date.UTC(Number(date.slice(0, 4)), Number(date.slice(5, 7)) - 1, Number(date.slice(8, 10)))
}

/**
 * What the release still needs, in the order its kind asks for it: the
 * roles, the score, the cover, the fields (v0.97). Empty exactly when the
 * release is ready. The order is the backend's, so the first gap a chip
 * shows is the first gap every other screen shows.
 */
export function missing(readiness: Readiness): Need[] {
  return readiness.checks.filter((check) => !check.done).map((check) => check.need)
}

/** One need as one stable string: a React key, and part of a complaint a
 *  dismissal remembers. */
export function needKey(need: Need): string {
  return need.kind === 'role' || need.kind === 'field' ? `${need.kind}:${need.key}` : need.kind
}

/** The release a need is read against: which kind of work it is, and which
 *  door it goes out through - the door's fields are what a field need names. */
export interface NeedOwner {
  work_kind: string
  kind: string
}

/**
 * A need in words, in one of two forms: `name` for a line of a checklist
 * ("Cover"), `missing` for the list after "Missing:" ("a cover"). A role
 * and a field are the profile's own words in both: the craft named them, and
 * a key the profile no longer names is said as the key rather than not said.
 */
export function sayNeed(
  need: Need,
  config: ProfileConfig,
  owner: NeedOwner,
  t: TFunction,
  form: 'name' | 'missing' = 'missing',
): string {
  const vocabulary = vocabularyOf(config, owner.work_kind)
  switch (need.kind) {
    case 'role':
      return labelOf(vocabulary.version_roles, need.key)
    case 'field': {
      const door = vocabulary.release_kinds.find((entry) => entry.key === owner.kind)
      return labelOf(door?.fields ?? [], need.key)
    }
    case 'score':
      return form === 'name' ? t('readiness.score') : t('calendar.missingScore')
    case 'cover':
      return form === 'name' ? t('readiness.cover') : t('calendar.missingCover')
  }
}

/** Where on a work a need is answered: a tab of its card. */
export type AnsweredOn = 'versions' | 'score' | 'cover' | 'files' | 'overview'

/**
 * The tab of a work that closes a gap: a role on its versions, the score on
 * the score, a field where the release is written - the overview, the
 * release's one home since v0.90 - and the cover where it is made: the
 * Cover tab of a kind that builds one, the Files of any other, where a
 * picture is attached and set as the cover.
 */
export function answeredOn(need: Need, config: ProfileConfig, workKind: string): AnsweredOn {
  switch (need.kind) {
    case 'role':
      return 'versions'
    case 'score':
      return 'score'
    case 'field':
      return 'overview'
    case 'cover':
      return vocabularyOf(config, workKind).cover ? 'cover' : 'files'
  }
}
