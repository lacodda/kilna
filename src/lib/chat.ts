import type { Applied, ChatSummary, Message, Proposal, Run } from '@/lib/api/types'
import { inOrder, view, type RunView } from '@/lib/runs'

/**
 * One exchange as the conversation draws it: what was asked, what the run did
 * on the way, and what came back.
 *
 * The transcript is the spine — messages are what survives forever — and runs
 * are an overlay on it: steps while one is going, a note when one ended
 * without an answer. Since v0.28 both sides of an exchange carry the run's id
 * in their meta, so pairing is exact; messages written before that are paired
 * by their text, oldest first, which is the best that can be done for them.
 */
export interface Exchange {
  key: string
  /** What was asked. Null for an answer whose question never made it to storage. */
  prompt: string | null
  /** The run behind the exchange, live or replayed, when one is known. */
  run: RunView | null
  /** The settled answer from the transcript. */
  answer: {
    id: string
    body: string
    cost: number | null
    /** How long the answer took, in milliseconds, when the CLI said. */
    durationMs: number | null
    /** What the answer proposed, when its action asked for something applicable. */
    proposal: Proposal | null
    /** Who made it, when it came from outside the window — an MCP client's name. */
    source: string | null
    /** What the proposer said about it, shown beside the proposal. */
    note: string | null
    /** What applying the proposal made, once somebody did. On the message,
     * not in the component: the mark has to survive the next fetch. */
    applied: Applied | null
    /** When somebody turned the proposal down, stamped on the message the
     * way `applied` is: a refused proposal is answered, and is not waiting. */
    dismissed: string | null
    /** Why the action's block could not become a proposal, when it could
     * not — said rather than dropped, so a missing button is never silent. */
    refused: string | null
  } | null
  at: string
}

const runId = (message: Message): string | null =>
  typeof message.meta.run_id === 'string' ? message.meta.run_id : null

/**
 * The structured result an answer carried, if any.
 *
 * Read from the stored message rather than parsed here: the profile it was
 * checked against lives in the backend, and a proposal shown live and one shown
 * on replay have to be the same thing.
 */
const proposalOf = (message: Message): Proposal | null => {
  const raw = message.meta.proposal
  if (raw === null || typeof raw !== 'object' || Array.isArray(raw)) return null
  const proposal = raw as Partial<Proposal>
  switch (proposal.kind) {
    case 'score':
      return typeof (proposal as Partial<{ axes: unknown }>).axes === 'object'
        ? (proposal as Proposal)
        : null
    case 'version':
      return typeof (proposal as Partial<{ role: unknown }>).role === 'string'
        ? (proposal as Proposal)
        : null
    case 'note':
      return proposal as Proposal
    case 'work':
      return proposal as Proposal
    case 'scenes':
      return Array.isArray((proposal as Partial<{ scenes: unknown }>).scenes)
        ? (proposal as Proposal)
        : null
    // Until v0.77 these three fell to the default: a drafted reply or a
    // comment read off a screenshot said nothing under its answer, and a
    // style's description had no button at all.
    case 'comment':
      return typeof (proposal as Partial<{ body: unknown }>).body === 'string'
        ? (proposal as Proposal)
        : null
    case 'reply':
      return typeof (proposal as Partial<{ comment_id: unknown }>).comment_id === 'string'
        ? (proposal as Proposal)
        : null
    case 'description':
      return typeof (proposal as Partial<{ style_id: unknown }>).style_id === 'string'
        ? (proposal as Proposal)
        : null
    // What a release goes out under (v0.86), field by field.
    case 'release':
      return typeof (proposal as Partial<{ release_id: unknown }>).release_id === 'string'
        ? (proposal as Proposal)
        : null
    // The canon's two (v0.84) fell to the default until v0.86: a package of
    // cards or a card's picture prompt said nothing under its answer.
    case 'canon':
      return typeof (proposal as Partial<{ package: unknown }>).package === 'object'
        ? (proposal as Proposal)
        : null
    case 'cardPrompt':
      return typeof (proposal as Partial<{ note_id: unknown }>).note_id === 'string'
        ? (proposal as Proposal)
        : null
    // Ideas for a cover (v0.89), put on its board.
    case 'coverIdeas':
      return Array.isArray((proposal as Partial<{ ideas: unknown }>).ideas)
        ? (proposal as Proposal)
        : null
    // Trials for an experiment (v0.95), put on its board.
    case 'trials':
      return Array.isArray((proposal as Partial<{ trials: unknown }>).trials)
        ? (proposal as Proposal)
        : null
    // Words for the record (v0.90, ADR 0052): the bank and its blocks, how a
    // word is sung, a term of the register, the works a meaning is in.
    case 'words':
      return Array.isArray((proposal as Partial<{ package: { words?: unknown } }>).package?.words)
        ? (proposal as Proposal)
        : null
    default:
      return null
  }
}

/** The mark an applied proposal carries, when it carries one. */
const appliedOf = (message: Message): Applied | null => {
  const raw = message.meta.applied
  if (raw === null || typeof raw !== 'object' || Array.isArray(raw)) return null
  return typeof (raw as Partial<Applied>).at === 'string' ? (raw as Applied) : null
}

/** The client a proposal came from, when it came from outside the window. */
const sourceOf = (message: Message): string | null =>
  message.meta.source === 'mcp' && typeof message.meta.client === 'string'
    ? message.meta.client
    : null

/** Fold a chat's transcript and its runs into one conversation, oldest first. */
export function conversation(messages: Message[], runs: Run[]): Exchange[] {
  const ordered = inOrder(runs)
  const byId = new Map(ordered.map((run) => [run.id, run]))
  const claimed = new Set<string>()
  const items: Exchange[] = []
  const slotByRun = new Map<string, Exchange>()

  // For a question stored without a run id: the oldest unclaimed run that
  // asked the same thing. Consumed one-to-one, so the same question asked
  // twice claims two different runs.
  const claimByText = (body: string): Run | null =>
    ordered.find((run) => !claimed.has(run.id) && run.prompt === body) ?? null

  for (const message of messages) {
    if (message.role === 'user') {
      const named = runId(message)
      const run = named !== null ? (byId.get(named) ?? null) : claimByText(message.body)
      if (run !== null) claimed.add(run.id)

      const item: Exchange = {
        key: message.id,
        prompt: message.body,
        run: run === null ? null : view(run),
        answer: null,
        at: message.created_at,
      }
      if (run !== null) slotByRun.set(run.id, item)
      items.push(item)
    } else if (message.role === 'assistant') {
      const answer = {
        id: message.id,
        body: message.body,
        cost: typeof message.meta.cost_usd === 'number' ? message.meta.cost_usd : null,
        durationMs: typeof message.meta.duration_ms === 'number' ? message.meta.duration_ms : null,
        proposal: proposalOf(message),
        source: sourceOf(message),
        note: typeof message.meta.note === 'string' ? message.meta.note : null,
        applied: appliedOf(message),
        dismissed: typeof message.meta.dismissed === 'string' ? message.meta.dismissed : null,
        refused:
          typeof message.meta.proposal_refused === 'string' ? message.meta.proposal_refused : null,
      }

      // The exchange this answers: named by run id, or — for the untagged
      // past — the latest question still waiting for one.
      const named = runId(message)
      const slot =
        (named === null ? undefined : slotByRun.get(named)) ??
        items.findLast((item) => item.prompt !== null && item.answer === null)

      if (slot !== undefined && slot.answer === null) {
        slot.answer = answer
      } else {
        items.push({ key: message.id, prompt: null, run: null, answer, at: message.created_at })
      }
    }
  }

  // A run whose question never reached the transcript — the write failed mid
  // run. Rare, but a run must never be invisible.
  for (const run of ordered) {
    if (claimed.has(run.id)) continue
    items.push({
      key: run.id,
      prompt: run.prompt,
      run: view(run),
      answer: null,
      at: run.started_at,
    })
  }

  // Stable sort: same-instant items keep the transcript's order.
  return items.sort((left, right) => (left.at < right.at ? -1 : left.at > right.at ? 1 : 0))
}

/**
 * The exchanges whose proposal nobody has answered yet — what *apply all*
 * would take. An answer still growing is not counted: its proposal is not
 * settled. Nor is one turned down: the backend's *apply all* leaves it alone,
 * and a count that included it would promise more than the button does.
 */
export function pending(items: Exchange[]): Exchange[] {
  return items.filter(
    (item) =>
      item.answer?.proposal != null &&
      item.answer.applied === null &&
      item.answer.dismissed === null &&
      item.run?.working !== true,
  )
}

/** What an answer's menu offers besides copying it. */
export interface Offers {
  /** Insert the answer as a version of the chat's work. */
  insert: boolean
  /** Keep the answer as a note. */
  keep: boolean
}

/**
 * What an answer can be kept as, from its menu.
 *
 * Nothing while it is still growing: half an answer is not worth keeping.
 * Nothing when the answer proposed something with a card of its own - a
 * version, a package, a storyboard - because the card keeps it, and keeping
 * a rendering of a package as a lyric would be a second, wrong copy. A comment
 * or a reply is kept on the comments, and a style's description onto its
 * brick; neither is a version or a note of anything. Inserting needs a work;
 * keeping a note does not, and is not offered on an answer that proposed a
 * note, which its card already keeps.
 */
export function offers(item: Exchange, onWork: boolean): Offers {
  const settled = item.run?.working !== true
  const kind = item.answer?.proposal?.kind
  const ownCard =
    kind === 'version' ||
    kind === 'work' ||
    kind === 'scenes' ||
    kind === 'comment' ||
    kind === 'reply' ||
    kind === 'description' ||
    kind === 'coverIdeas' ||
    kind === 'trials'
  return {
    insert: onWork && settled && !ownCard,
    keep: settled && !ownCard && kind !== 'note',
  }
}

/**
 * Which chat a surface shows.
 *
 * The one chosen, as long as the list still has it - or it is the one this
 * surface has just made, which the list has not caught up with yet: the list
 * is refetched after the write, and in the meantime the question just asked
 * would vanish into a blank chat. The latest otherwise, and none when there
 * are no chats, which is where the first question makes one.
 */
export function shownChat(
  list: readonly { id: string }[],
  chosen: string | null,
  made: string | null,
): string | null {
  if (chosen !== null && (chosen === made || list.some((chat) => chat.id === chosen))) {
    return chosen
  }
  return list[0]?.id ?? null
}

/** What the list calls a chat: its name, its first question, or the fallback. */
export function chatLabel(summary: ChatSummary, fallback: string): string {
  return summary.title ?? summary.first_prompt ?? fallback
}
