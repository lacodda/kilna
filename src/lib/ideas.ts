import type {
  CoverBoard,
  IdeaCard,
  IdeaRequest,
  ProfileConfig,
  PromptTemplate,
  SiblingCover,
} from '@/lib/api/types'

/**
 * The board of ideas for a publication's cover (v0.89, ADR 0050), as the
 * window reads it: which cards a filter shows, what a task about the board is
 * called, how many ideas a "Make…" asks for.
 */

/** The most ideas one run is asked for - the backend's `idea::MOST`. */
export const MOST_IDEAS = 5

/** What a "Make…" asks for when the profile says nothing - `idea::ON_MAKE`. */
export const IDEAS_ON_MAKE = 3

/** One card of the board: an idea on it, or a neighbour's cover offered. */
export type BoardCard =
  { kind: 'idea'; card: IdeaCard } | { kind: 'sibling'; sibling: SiblingCover }

/** The board's filters, in the mockup's order. */
export const BOARD_FILTERS = ['all', 'star', 'mine', 'sibling', 'rejected'] as const
export type BoardFilter = (typeof BOARD_FILTERS)[number]

/**
 * Every card of a board: the ideas newest first - a run's answer lands where
 * its skeletons stood - and after them the neighbours' covers not yet copied.
 */
export function cardsOf(board: CoverBoard): BoardCard[] {
  const ideas: BoardCard[] = [...board.ideas].reverse().map((card) => ({ kind: 'idea', card }))
  const siblings: BoardCard[] = board.siblings.map((sibling) => ({ kind: 'sibling', sibling }))
  return [...ideas, ...siblings]
}

/** Whether a card shows under a filter. A turned-down idea stays under "all",
 *  dimmed: the board is where it was judged, and it is still an example. */
export function shows(card: BoardCard, filter: BoardFilter): boolean {
  if (card.kind === 'sibling') return filter === 'all' || filter === 'sibling'
  const { source, verdict } = card.card.idea
  switch (filter) {
    case 'all':
      return true
    case 'star':
      return verdict === 'star'
    case 'mine':
      return source === 'own' || source === 'refined'
    case 'sibling':
      return source === 'sibling'
    case 'rejected':
      return verdict === 'rejected'
  }
}

/** How many cards each filter shows. */
export function counts(cards: readonly BoardCard[]): Record<BoardFilter, number> {
  const out = { all: 0, star: 0, mine: 0, sibling: 0, rejected: 0 }
  for (const card of cards) {
    for (const filter of BOARD_FILTERS) if (shows(card, filter)) out[filter] += 1
  }
  return out
}

/** The ideas on the shortlist, oldest first - the constructor's chips. */
export function shortlist(board: CoverBoard): IdeaCard[] {
  return board.ideas.filter((card) => card.idea.verdict === 'star')
}

/** The profile's action that proposes ideas for a cover of this kind. */
export function coverActionOf(
  config: ProfileConfig,
  kind: string | undefined,
): PromptTemplate | undefined {
  return config.prompts.find(
    (prompt) =>
      prompt.scope === 'cover' &&
      prompt.produces === 'cover-ideas' &&
      ((prompt.kinds ?? []).length === 0 || (kind !== undefined && prompt.kinds!.includes(kind))),
  )
}

/** The key a board's task runs under - the backend's `cover_key`. */
export function coverTaskKey(action: string, workId: string): string {
  return `${action}:cover:${workId}`
}

/** The publication a task key is about, when it is a board's. */
export function boardOfTask(key: string | undefined): string | null {
  if (key === undefined) return null
  const [, scope, workId] = key.split(':')
  return scope === 'cover' && workId !== undefined && workId !== '' ? workId : null
}

/** How many ideas "Make…" asks for. */
export function ideasOnMake(config: ProfileConfig): number {
  return Math.min(Math.max(config.cover_ideas ?? IDEAS_ON_MAKE, 0), MOST_IDEAS)
}

/** What the composer asks for: `count` of the assistant's own, and the
 *  person's words worked out when they asked for that and wrote any. */
export function requestOf(count: number, words: string, refine: boolean): IdeaRequest {
  const own = words.trim()
  return { count, refine: refine && own !== '' ? own : null, more: false }
}

/** How many ideas a request makes - the skeletons drawn while it runs. */
export function totalOf(request: IdeaRequest): number {
  return request.count + (request.refine !== null && request.refine.trim() !== '' ? 1 : 0)
}
