import { useQuery } from '@tanstack/react-query'
import i18n from '@/i18n'
import type { Guard, ProfileConfig, RepeatFinding, RepeatLevel, RepeatMark } from '@/lib/api/types'
import { formatDay } from '@/lib/format'
import { today } from '@/lib/month'
import { queries } from '@/lib/query/queries'

/*
 * The guard of repeats as the window draws it (ADR 0054).
 *
 * The backend reads every finding off the texts and the calendar each time it
 * is asked and stores nothing but the owner's "I know, I am keeping it"; this
 * module is what the screens share on top of that answer: one map of every
 * work's mark, read once for the whole workspace and looked up by every row,
 * chip and header that wears one, and the sentence a mark is explained by -
 * so the catalogue, the calendar and the card say a repeat in one voice.
 */

const current = () => i18n.resolvedLanguage ?? 'en'

/** Keyed by the work that wears it. Module-level, so the query's `select` is
 *  one function and TanStack keeps the map until the answer changes, rather
 *  than every chip on the month building its own. */
const byWork = (marks: RepeatMark[]): ReadonlyMap<string, RepeatMark> =>
  new Map(marks.map((mark) => [mark.work_id, mark]))

const NONE: ReadonlyMap<string, RepeatMark> = new Map()

/**
 * Every work's mark, by work id: a song its own, a publication its song's.
 *
 * Read on the user's own day - the window of "recent" is counted from it, and
 * the backend knows only UTC, which after sunset at a negative offset is
 * already tomorrow. Empty until the answer lands: a row without a mark is
 * what a workspace with nothing repeated looks like, so a mark that arrives a
 * moment late is not a flash of something wrong.
 */
function useRepeatMarks(): ReadonlyMap<string, RepeatMark> {
  const { data } = useQuery({ ...queries.repeatMarks(today()), select: byWork })
  return data ?? NONE
}

/** One work's mark, if it wears one. */
export function useRepeatMark(workId: string): RepeatMark | undefined {
  return useRepeatMarks().get(workId)
}

/**
 * The status hue a level is drawn in: red is the `bad` hue, orange the `warn`
 * one, the same pairs `Badge` and `StatusDot` use, so a repeat reads as loud
 * as anything else on the screen that wears those colours.
 */
export function statusOf(level: RepeatLevel): 'bad' | 'warn' {
  return level === 'red' ? 'bad' : 'warn'
}

/**
 * One finding in a sentence: the word, the other song, and the day it went
 * out or is booked for - "пульсар — in “Tide”, out Sep 20". A spent term of
 * the register says so first, since it is spent whenever it was said and the
 * day is only where it was last heard.
 */
export function repeatHint(
  finding: Pick<RepeatFinding, 'word' | 'why' | 'neighbour_title' | 'day' | 'booked'> &
    Partial<Pick<RepeatFinding, 'also'>>,
  language: string = current(),
): string {
  const when = finding.booked ? 'booked' : 'out'
  const key = finding.why === 'register' ? `repeats.hint.spent.${when}` : `repeats.hint.${when}`
  const hint = i18n.t(key, {
    lng: language,
    word: finding.word,
    title: finding.neighbour_title,
    day: formatDay(finding.day, language),
  })
  const also = finding.also ?? 0
  return also > 0 ? i18n.t('repeats.also', { lng: language, hint, count: also }) : hint
}

/**
 * What a mark says to anyone who does not see its colour: how loud it is, the
 * loudest finding, and how many more stand behind it. The accessible name of
 * every mark and the first line of its tooltip.
 */
export function markLabel(
  mark: Pick<RepeatMark, 'level' | 'top' | 'count'>,
  language: string = current(),
): string {
  const level = i18n.t(`repeats.level.${mark.level}`, { lng: language })
  const hint = repeatHint(mark.top, language)
  const more = mark.count - 1
  return more > 0
    ? i18n.t('repeats.markMore', { lng: language, level, hint, count: more })
    : i18n.t('repeats.mark', { lng: language, level, hint })
}

/**
 * A song's findings in the order they are read: the ones still counted before
 * the ones the owner kept, red before orange - and otherwise the backend's
 * order, which puts the most recent neighbour first. A kept finding stays on
 * the list (it is where "count it again" is), at the foot of it, because it
 * no longer asks for anything.
 */
export function orderFindings(findings: readonly RepeatFinding[]): RepeatFinding[] {
  const weight = (finding: RepeatFinding) =>
    (finding.kept ? 2 : 0) + (finding.level === 'red' ? 0 : 1)
  return findings
    .map((finding, index) => ({ finding, index }))
    .sort((a, b) => weight(a.finding) - weight(b.finding) || a.index - b.index)
    .map(({ finding }) => finding)
}

/**
 * What a release landing on a day would repeat, in one line for the day's
 * foot or a toast: the loudest finding, and how many more. `null` when it
 * repeats nothing - the day then says nothing about it.
 */
export function landingWarning(
  repeats: readonly RepeatFinding[],
  language: string = current(),
): string | null {
  const [first] = repeats
  if (first === undefined) return null
  const hint = repeatHint(first, language)
  const more = repeats.length - 1
  return more > 0
    ? i18n.t('repeats.tooCloseMore', { lng: language, hint, count: more })
    : i18n.t('repeats.tooClose', { lng: language, hint })
}

/** The guard as kilna reads it when the profile says nothing: 90 days, past
 *  the 20 000th stem of the language, in no more than two of the owner's
 *  works. The backend's defaults (`profile/config.rs`), said again here so
 *  the settings can show them. */
export const GUARD_DEFAULTS: Readonly<Guard> = {
  window_days: 90,
  rare_rank: 20_000,
  rare_in_works: 2,
}

export type GuardField = keyof Guard

/** The guard a profile reads, with the defaults where it names none. */
export function guardOf(config: Pick<ProfileConfig, 'guard'>): Guard {
  return { ...GUARD_DEFAULTS, ...config.guard }
}

/**
 * The profile's guard with one number changed.
 *
 * Whole numbers of at least one: a window of no days, a rank of nothing and a
 * word rare in no works are not settings, they are the guard switched off by
 * accident. And `null` - absent - when what is left says only the defaults,
 * so a profile tuned and tuned back is the document it was, and a later
 * change to kilna's defaults reaches it.
 */
export function guardWith(
  config: Pick<ProfileConfig, 'guard'>,
  field: GuardField,
  value: number,
): Guard | null {
  const next: Guard = { ...guardOf(config), [field]: Math.max(1, Math.trunc(value)) }
  const fields = Object.keys(GUARD_DEFAULTS) as GuardField[]
  return fields.every((key) => next[key] === GUARD_DEFAULTS[key]) ? null : next
}
