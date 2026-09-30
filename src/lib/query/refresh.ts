import type { QueryKey } from '@tanstack/react-query'
import { keys } from '@/lib/query/keys'

/**
 * What a write disturbs, by what it wrote.
 *
 * Each list names the query areas that read the kind of row a write changes,
 * so a mutation says what it changed - `refresh: refresh.note` - rather than
 * which screens happen to show it today. Until v0.77 twenty-nine mutations
 * kept a list of their own, and a screen added later was in some of them.
 *
 * The journal is not here: every write a person makes is in the log (ADR
 * 0033) and may add a line to the feed, so `useAppMutation` refreshes it after
 * every write, whatever the list.
 */
export const refresh = {
  /** A work's fields, stage, star, tags or tier: the lists that show works. */
  work: [keys.works, keys.catalogue] as readonly QueryKey[],
  /** A work made or gone: everything that counts or places works. */
  works: [
    keys.works,
    keys.catalogue,
    keys.workspace,
    keys.calendar,
    keys.register,
  ] as readonly QueryKey[],
  /** A version of one work: its list, the work that points at its current
   *  one, and the calendar - a release's readiness asks for a draft, and the
   *  catalogue's Ready column and the calendar's chips read it from there. */
  version: (workId: string): readonly QueryKey[] => [
    keys.versions(workId),
    keys.work(workId),
    keys.works,
    keys.calendar,
    keys.register,
  ],
  /** A score: every screen that ranks or tiers works, and the calendar, whose
   *  readiness asks for a score. */
  score: (workId: string): readonly QueryKey[] => [
    keys.scores,
    keys.work(workId),
    keys.works,
    keys.catalogue,
    keys.calendar,
  ],
  release: [keys.releases, keys.calendar, keys.releaseQueue, keys.catalogue] as readonly QueryKey[],
  note: [keys.notes, keys.tags] as readonly QueryKey[],
  /** A term of the register: the list, its works, and every text checked
   *  against it. */
  term: [keys.register] as readonly QueryKey[],
  comment: [keys.comments] as readonly QueryKey[],
  /** A comment or a reply kept from an answer: the inbox and the proposal it was. */
  keptComment: [keys.comments, keys.pendingProposals, keys.transcripts] as readonly QueryKey[],
  scene: [keys.scenes] as readonly QueryKey[],
  cut: [keys.cuts] as readonly QueryKey[],
  link: [keys.links] as readonly QueryKey[],
  style: [keys.styles] as readonly QueryKey[],
  /** The canon: a card's facts, relations, pictures or description. The
   *  notes too - a card is a note - and the search, which finds facts. */
  canon: [keys.canon, keys.notes, ['search']] as readonly QueryKey[],
  focus: [keys.focus] as readonly QueryKey[],
  profile: [keys.workspace, keys.profiles] as readonly QueryKey[],
}
