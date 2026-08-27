import type { ChatSummary, Message, Run, ScoreProposal } from '@/lib/api'
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
    /** What the answer proposed, when its action asked for something applicable. */
    proposal: ScoreProposal | null
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
const proposalOf = (message: Message): ScoreProposal | null => {
  const raw = message.meta.proposal
  if (raw === null || typeof raw !== 'object' || Array.isArray(raw)) return null
  const proposal = raw as Partial<ScoreProposal>
  return proposal.kind === 'score' && typeof proposal.axes === 'object'
    ? (proposal as ScoreProposal)
    : null
}

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
        proposal: proposalOf(message),
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

/** What the list calls a chat: its name, its first question, or the fallback. */
export function chatLabel(summary: ChatSummary, fallback: string): string {
  return summary.title ?? summary.first_prompt ?? fallback
}
