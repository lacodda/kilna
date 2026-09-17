import type { Cut, Shot } from '@/lib/api'

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

/** One stretch as the track draws it: where it sits on the donor, 0..1. */
export interface Band {
  cut: Cut
  /** How far along the donor it starts, 0..1. */
  left: number
  /** How much of the donor it covers, 0..1. */
  width: number
}

/**
 * The stretches of one donor, placed on that donor's length.
 *
 * `null` when the donor has no duration: a track drawn against an unknown
 * length would put a 12-second cut at an arbitrary place and look exactly as
 * authoritative as a real one. The screen says the length is missing instead,
 * which is a thing a person can fix.
 *
 * Stretches are placed, not scaled to fit: two cuts eight minutes apart in a
 * long video should read as two marks far apart, because that is what they
 * are.
 */
export function bandsOf(cuts: Cut[], duration: number | null): Band[] | null {
  if (duration === null || !Number.isFinite(duration) || duration <= 0) return null
  return cuts.map((cut) => ({
    cut,
    left: clamp(cut.starts_at / duration),
    // Clamped at the left edge too, so a stretch that runs past the end of a
    // donor whose length was shortened later still draws inside the track
    // rather than overflowing the row.
    width: clamp(Math.min(cut.ends_at, duration) / duration) - clamp(cut.starts_at / duration),
  }))
}

function clamp(value: number): number {
  return Math.min(1, Math.max(0, value))
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

/** What stops a splice from being cut into a file, or nothing. */
export type Blocker = 'empty' | 'noFile'

/**
 * Whether the plugin could cut this short right now.
 *
 * Two things stop it, and they are different problems with different fixes:
 * a splice with no stretches has nothing to cut, and a splice whose donor has
 * no video attached has nowhere to cut it from. Both are ordinary states of
 * work in progress, which is why this reports rather than refuses.
 */
export function blockerOf(shots: Shot[]): Blocker | null {
  if (shots.length === 0) return 'empty'
  if (shots.some((shot) => shot.path === null)) return 'noFile'
  return null
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
 */
export function canBeCut(cuts: Cut[], donors: number): boolean {
  return cuts.length > 0 || donors > 0
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
