import type { Dismissal, ProfileConfig, ScheduledRelease, ScoredWork } from '@/lib/api'
import { daysBetween } from '@/lib/readiness'

/**
 * What the workspace notices about itself.
 *
 * A finding is a complaint with a subject: *this* work, or *this* release, and
 * what is off about it. Nothing here writes anything — a finding is read, and
 * the only thing offered alongside it is a profile action, which lands in a
 * chat like any other. The assistant proposes; the person applies.
 *
 * **Only what can still be done.** The rule comes from the predecessor getting
 * it wrong: its first weak-scheduled finding listed ten works that had already
 * gone out, offering to pull published releases off the calendar. Every finding
 * below therefore excludes what is finished — released work, past dates, work
 * that has been shelved.
 *
 * The dashboard reads the same two lists for its own sections, and two of these
 * kinds overlap with it on purpose: the dashboard answers "what needs me today",
 * a finding is a standing complaint that outlives the day.
 *
 * Nothing here is stored. A finding is derived on every read and leaves on its
 * own the moment its complaint stops being true — which is why the board has no
 * pin for one and no order of its own: there is nothing to keep. What the person
 * decides *about* a finding is the one thing derivation cannot know, and
 * [`visible`] is where that decision is applied.
 */

/** What kind of complaint this is. The key is stable — v0.34 stores it. */
export type FindingKind =
  /** Nothing has judged it, so nothing can rank it. */
  | 'unscored'
  /** The score describes a draft that has since been rewritten. */
  | 'stale-score'
  /** Judged and ready, but nothing is booked. */
  | 'ready-unscheduled'
  /** The weakest tier holding a slot while stronger work waits. */
  | 'weak-scheduled'
  /** A draft nobody has touched in a long time. */
  | 'stale-draft'

export interface Finding {
  kind: FindingKind
  /** The work this is about. */
  workId: string
  title: string
  /**
   * What the complaint *is*, as a stable string.
   *
   * v0.34 hides a finding by remembering this: the same complaint stays quiet,
   * a changed one is news again. "missing: cover" and "missing: cover, audio"
   * are different complaints about the same work, and the second deserves to
   * be seen even if the first was dismissed.
   */
  complaint: string
  /** The profile action that would answer it, when one does. */
  action?: string
}

/** How long a draft may sit untouched before it counts as stalled. */
export const STALE_DRAFT_DAYS = 30

/** What a work must have gone through to be worth chasing. */
function isOpen(work: ScoredWork): boolean {
  return work.released === 0
}

/**
 * Everything worth mentioning, in a stable order.
 *
 * `actions` are the profile's prompt keys; a finding only offers one that
 * actually exists, so a profile that dropped its scoring action simply gets a
 * finding with nothing attached rather than a button that fails.
 */
export function findings(
  works: readonly ScoredWork[],
  calendar: readonly ScheduledRelease[],
  config: Pick<ProfileConfig, 'tiers' | 'prompts'>,
  today: string,
): Finding[] {
  const actions = new Set(config.prompts.map((prompt) => prompt.key))
  const offer = (key: string) => (actions.has(key) ? key : undefined)

  const found: Finding[] = []

  for (const work of works) {
    if (!isOpen(work)) continue

    if (work.total === null) {
      found.push({
        kind: 'unscored',
        workId: work.work_id,
        title: work.title,
        complaint: 'unscored',
        action: offer('score'),
      })
      // An unscored work cannot also have a stale score, and chasing it to
      // publish something nothing has judged would be the wrong advice.
      continue
    }

    if (work.stale) {
      found.push({
        kind: 'stale-score',
        workId: work.work_id,
        title: work.title,
        // The date is part of the complaint: rewriting it again is news.
        complaint: `stale-score:${work.scored_at ?? ''}`,
        action: offer('score'),
      })
    }

    if (work.scheduled === 0) {
      found.push({
        kind: 'ready-unscheduled',
        workId: work.work_id,
        title: work.title,
        complaint: 'ready-unscheduled',
      })
    }

    const idle = daysBetween(work.updated_at.slice(0, 10), today)
    if (idle >= STALE_DRAFT_DAYS && work.scheduled === 0) {
      found.push({
        kind: 'stale-draft',
        workId: work.work_id,
        title: work.title,
        // Bucketed by month rather than exact days: a complaint that changes
        // every morning would resurface every morning once hidden.
        complaint: `stale-draft:${Math.floor(idle / 30)}`,
        action: offer('polish') ?? offer('critique'),
      })
    }
  }

  found.push(...weakScheduled(works, calendar, config, today))

  // Stable across renders and across runs: the same workspace produces the
  // same order, which is what lets v0.34 tell a new finding from a moved one.
  return found.sort(
    (a, b) => a.kind.localeCompare(b.kind) || a.title.localeCompare(b.title),
  )
}

/**
 * Slots held by the weakest tier while stronger work waits its turn.
 *
 * Only worth saying when there is something to swap in: "this is weak" with
 * nothing better available is a complaint about the work, not about the plan,
 * and the calendar already shows the score.
 */
function weakScheduled(
  works: readonly ScoredWork[],
  calendar: readonly ScheduledRelease[],
  config: Pick<ProfileConfig, 'tiers'>,
  today: string,
): Finding[] {
  const weakest = [...config.tiers].sort((a, b) => a.min - b.min)[0]
  if (weakest === undefined) return []

  // What could take the slot instead: judged, unbooked, and stronger than the
  // floor. Without one of these the finding has no advice to give.
  const waiting = works.some(
    (work) =>
      isOpen(work) && work.scheduled === 0 && work.tier !== null && work.tier !== weakest.key,
  )
  if (!waiting) return []

  return calendar
    .filter((entry) => entry.released_at === null && entry.scheduled_at !== null)
    // A date that has passed cannot be given to anyone else.
    .filter((entry) => daysBetween(today, entry.scheduled_at as string) >= 0)
    .filter((entry) => entry.tier === weakest.key)
    // A date settled by hand is a decision, not an oversight.
    .filter((entry) => entry.slot_pinned_at === null)
    .map((entry) => ({
      kind: 'weak-scheduled' as const,
      workId: entry.work_id,
      title: entry.work_title,
      complaint: `weak-scheduled:${entry.scheduled_at ?? ''}`,
    }))
}

/**
 * What is left after the person has answered some of it.
 *
 * A dismissal remembers the *complaint*, not the work and not the kind. The
 * same complaint stays quiet; a changed one is news again. That distinction is
 * the whole reason v0.33 rounded the unstable complaints to months — a
 * complaint that changed every morning would come back every morning, and
 * hiding would mean nothing.
 */
export function visible(found: readonly Finding[], dismissed: readonly Dismissal[]): Finding[] {
  const answered = new Set(
    dismissed.map((row) => keyOf(row.kind, row.work_id, row.complaint)),
  )
  return found.filter(
    (finding) => !answered.has(keyOf(finding.kind, finding.workId, finding.complaint)),
  )
}

/**
 * The three things that identify a complaint, as one string.
 *
 * Joined on a separator no id, kind or complaint can hold. A complaint carries
 * a date or a count, and a separator one of them could contain would let two
 * different complaints collapse into the same key — hiding one would silently
 * hide the other.
 */
function keyOf(kind: string, workId: string, complaint: string): string {
  return [kind, workId, complaint].join(SEPARATOR)
}

/** A control character, so it cannot appear in an id or a complaint. */
const SEPARATOR = '\u0001'

/** What `dismiss_finding` needs to remember this one. */
export function dismissalKey(finding: Finding) {
  return { kind: finding.kind, work_id: finding.workId, complaint: finding.complaint }
}
