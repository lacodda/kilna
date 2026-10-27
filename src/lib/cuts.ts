import type { Cut, Shot } from '@/lib/api/types'

/**
 * The arithmetic of a splice: what a track draws, and whether it can be cut.
 *
 * Kept out of the component for the reason `scenes.ts` gives — a rule with a
 * test beside it, rather than a condition buried three levels into JSX.
 */

/** How long a stretch runs. */
export function lengthOf(cut: Cut): number {
  return cut.ends_at - cut.starts_at
}

/** How long the finished short runs: every stretch, added up. */
export function totalLength(cuts: Cut[]): number {
  return cuts.reduce((sum, cut) => sum + lengthOf(cut), 0)
}

/**
 * The length a donor's track is drawn against, or `null` when there is none.
 *
 * A track drawn against an unknown length would put a 12-second cut at an
 * arbitrary place and look exactly as authoritative as a real one. The screen
 * says the length is missing instead, which is a thing a person can fix.
 *
 * Where each stretch then sits is dowel's Track (`track-segments`): placed on
 * the donor's whole length rather than scaled to fit, so two cuts eight
 * minutes apart in a long video read as two marks far apart, and clamped to
 * the track, so a stretch past the end of a donor shortened later still draws
 * inside the row.
 */
export function scaleOf(duration: number | null): number | null {
  if (duration === null || !Number.isFinite(duration) || duration <= 0) return null
  return duration
}

/** Where a stretch runs on its donor, in seconds. */
export interface Stretch {
  starts_at: number
  ends_at: number
}

/** What a drag on the track holds: the whole stretch, or one of its ends. */
export type Grip = 'whole' | 'start' | 'end'

/** The shortest a drag may leave a stretch: still wide enough to take hold of
    again, and longer than any cut anyone means. Typed seconds are not held to
    it - the keyboard is for the exact value. */
export const SHORTEST_DRAG = 0.5

/**
 * Where a stretch lands when its track is dragged `delta` seconds.
 *
 * The step is taken in tenths, the finest a timecode shows: a drag lands on a
 * value the field under the track can say, and a press that did not move far
 * enough to matter changes nothing - no write, no line in the history.
 *
 * Held inside the donor. The whole stretch keeps its length and stops at
 * either end; an end stops at the donor's edge and short of the other end.
 * The backend refuses a stretch that ends before it starts, and a drag that
 * got there would be a failure toast for a gesture that only went too far.
 */
export function dragged(stretch: Stretch, grip: Grip, delta: number, duration: number): Stretch {
  const step = Math.round(delta * 10) / 10
  if (step === 0) return stretch
  const { starts_at, ends_at } = stretch

  if (grip === 'whole') {
    const length = ends_at - starts_at
    const starts = clamp(starts_at + step, 0, Math.max(0, duration - length))
    return { starts_at: tidy(starts), ends_at: tidy(starts + length) }
  }
  if (grip === 'start') {
    return { starts_at: tidy(clamp(starts_at + step, 0, ends_at - SHORTEST_DRAG)), ends_at }
  }
  const last = Math.max(duration, starts_at + SHORTEST_DRAG)
  return { starts_at, ends_at: tidy(clamp(ends_at + step, starts_at + SHORTEST_DRAG, last)) }
}

/**
 * A dragged stretch with the end that moved on the nearest place a part of
 * the donor begins, when one lies within `within` seconds (v0.103).
 *
 * People start a short where a part starts - a stretch that opens mid-phrase
 * sounds cut rather than made - and on the owner's shorts nearly every start
 * lies within a few seconds of where a part was heard to begin. So an edge
 * let go near a beginning lands on it. A whole stretch lands by its start, or
 * else by its end, keeping its length; an end never crosses the other.
 */
export function snapped(
  stretch: Stretch,
  grip: Grip,
  marks: readonly number[],
  within: number,
): Stretch {
  const nearest = (at: number): number | null => {
    let best: number | null = null
    for (const mark of marks) {
      const off = Math.abs(mark - at)
      if (off <= within && (best === null || off < Math.abs(best - at))) best = mark
    }
    return best
  }
  const { starts_at, ends_at } = stretch
  if (grip === 'start') {
    const mark = nearest(starts_at)
    return mark === null || mark > ends_at - SHORTEST_DRAG
      ? stretch
      : { starts_at: tidy(mark), ends_at }
  }
  if (grip === 'end') {
    const mark = nearest(ends_at)
    return mark === null || mark < starts_at + SHORTEST_DRAG
      ? stretch
      : { starts_at, ends_at: tidy(mark) }
  }
  const length = ends_at - starts_at
  const start = nearest(starts_at)
  if (start !== null) return { starts_at: tidy(start), ends_at: tidy(start + length) }
  const end = nearest(ends_at)
  if (end !== null && end - length >= 0)
    return { starts_at: tidy(end - length), ends_at: tidy(end) }
  return stretch
}

/** How much of `stretch` lies inside `other`, as a share of `stretch`. */
export function shareInside(stretch: Stretch, other: Stretch): number {
  const length = stretch.ends_at - stretch.starts_at
  if (length <= 0) return 0
  const inside =
    Math.min(stretch.ends_at, other.ends_at) - Math.max(stretch.starts_at, other.starts_at)
  return Math.max(0, inside) / length
}

function clamp(value: number, low: number, high: number): number {
  return Math.min(high, Math.max(low, value))
}

/** Seconds without the float noise of adding tenths: 12.3, not
    12.300000000000001, is what gets stored. */
function tidy(seconds: number): number {
  return Math.round(seconds * 1000) / 1000
}

/**
 * The splice grouped by the video each stretch comes from, donors in the order
 * they first appear.
 *
 * A short is usually cut from one video and draws one track; it may be cut
 * from two, and then it draws two. The grouping is here rather than in the
 * component because "which donors, in what order" is the same question the
 * calendar's spacing rule asks.
 */
export interface Track {
  source_id: string
  source_title: string
  source_duration: number | null
  cuts: Cut[]
}

export function tracksOf(cuts: Cut[]): Track[] {
  const tracks: Track[] = []
  for (const cut of cuts) {
    const found = tracks.find((track) => track.source_id === cut.source_id)
    if (found) {
      found.cuts.push(cut)
      continue
    }
    tracks.push({
      source_id: cut.source_id,
      source_title: cut.source_title,
      source_duration: cut.source_duration,
      cuts: [cut],
    })
  }
  return tracks
}

/** What stops a splice from being cut into a file, or nothing: no
 *  stretches, or a donor whose finished file is not to be had - its folder
 *  is not there, nothing in it matches its kind's rule yet, its kind names
 *  no rule, or the file chosen for it is gone (ADR 0070). */
export type Blocker = 'empty' | 'noFolder' | 'notYet' | 'noRule' | 'chosenMissing'

/**
 * Whether the plugin could cut this short right now.
 *
 * Different problems with different fixes: a splice with no stretches has
 * nothing to cut, and a splice whose donor has no finished file has nowhere
 * to cut it from - and why it has none says where to look. All are ordinary
 * states of work in progress, which is why this reports rather than refuses.
 */
export function blockerOf(shots: Shot[]): Blocker | null {
  if (shots.length === 0) return 'empty'
  const missing = shots.find((shot) => shot.path === null)
  if (missing === undefined) return null
  switch (missing.file_state) {
    case 'notYet':
    case 'noRule':
    case 'chosenMissing':
      return missing.file_state
    // A file found or chosen always has a path; a line without one is a
    // folder that could not be looked in.
    default:
      return 'noFolder'
  }
}

/**
 * Whether the card draws the splice: the work has stretches, or a donor it
 * could take them from.
 *
 * A fact about this work, not a rule about its kind — deliberately. The code
 * is not allowed to know that `short` means a short (ADR 0001), and no test
 * over the vocabulary tells a kind that gets cut out of videos from one that
 * does not: `derive_work` lets any kind be made from any kind, and a studio
 * that cuts trailers out of films is doing the same thing with other words.
 *
 * So the question the card asks is the one it can answer honestly. A work
 * with a donor is one somebody said was made from another, which is exactly
 * when "which parts of it" becomes a question; a work with stretches already
 * shows the tab whatever else is true, because hiding the screen that edits
 * them would strand the data.
 *
 * Numbers rather than lists: the card reads both from its counters
 * (`card_counts`), and fetches no splice for a work that has none.
 */
export function canBeCut(stretches: number, donors: number, cutFrom = 0): boolean {
  // A donor's tab names what was cut out of it: a film only ever cut from
  // has something on that tab too.
  return stretches > 0 || donors > 0 || cutFrom > 0
}

/**
 * The order a drag leaves the splice in: `moving` taken out and put back
 * before `before`, or at the end when that is null.
 *
 * The whole order is returned because that is what the backend takes — one
 * change, one undo. Lifted from `scenes.ts`'s `orderMoving`, which does the
 * same job for a board; not shared with it, because a helper over "a list of
 * things with ids" is a helper that hides which list.
 */
export function orderMoving(cuts: Cut[], moving: string, before: string | null): string[] {
  const ids = cuts.map((cut) => cut.id).filter((id) => id !== moving)
  if (before === null) return [...ids, moving]
  const at = ids.indexOf(before)
  if (at === -1) return [...ids, moving]
  return [...ids.slice(0, at), moving, ...ids.slice(at)]
}

/**
 * The order of the whole splice after one donor's list was rearranged:
 * `moving` put at index `to` of `track`, the stretches of that donor alone.
 *
 * The list under a track shows one donor, while the order the backend keeps
 * runs across all of them. Moved before the stretch that now follows it in
 * its own list, or - moved to the bottom - just after the donor's last one,
 * rather than after the last stretch of the whole splice, which could belong
 * to another donor and would carry this one out of its place among its own.
 */
export function orderWithin(splice: Cut[], track: Cut[], moving: string, to: number): string[] {
  const rest = track.map((cut) => cut.id).filter((id) => id !== moving)
  const before = rest[to]
  if (before !== undefined) return orderMoving(splice, moving, before)

  const others = splice.map((cut) => cut.id).filter((id) => id !== moving)
  const last = rest.at(-1)
  const after = last === undefined ? -1 : others.indexOf(last)
  return orderMoving(splice, moving, after === -1 ? null : (others[after + 1] ?? null))
}
