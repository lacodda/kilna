import type { Run, RunEvent } from '@/lib/api'

/**
 * What the panel shows for a run, whether it is going or long over.
 *
 * A run is a list of events, and the same list has to read the same way twice:
 * live, as each event arrives, and replayed from storage when the panel comes
 * back to a chat it left. Folding both through one function is what makes those
 * two views agree — the live case is just the replay of a list that is still
 * growing.
 */
export interface RunView {
  id: string
  /** What was asked. Shown as the question above the answer. */
  prompt: string
  /** Tools used, in order, as short captions. */
  steps: string[]
  /** The answer so far, blocks joined. */
  body: string
  /** Set when the run ended badly — the CLI's own wording where there is one. */
  failure: string | null
  /** Set when the run was stopped by hand, which is not a failure. */
  cancelled: boolean
  working: boolean
  cost: number | null
  startedAt: string
}

/** Fold a run's events into what the panel draws. */
export function view(run: Run): RunView {
  const steps: string[] = []
  const blocks: string[] = []
  let failure: string | null = null
  let cost: number | null = null

  for (const event of run.events) {
    switch (event.kind) {
      case 'tool':
        steps.push(event.detail === '' ? event.name : `${event.name} · ${event.detail}`)
        break
      case 'text':
        blocks.push(event.body)
        break
      case 'finished':
        // The final body repeats what the text blocks already said, so it is
        // used only when nothing was streamed — a short answer can arrive as a
        // result and nothing else.
        if (blocks.length === 0 && event.body.trim() !== '') blocks.push(event.body)
        cost = event.cost_usd ?? null
        break
      case 'failed':
        failure = event.message
        break
      case 'started':
      case 'stopped':
        break
    }
  }

  return {
    id: run.id,
    prompt: run.prompt,
    steps,
    body: blocks.join('\n\n'),
    // A run the app abandoned has no failure event — its row carries the reason.
    failure: failure ?? (run.state === 'broken' ? (run.detail ?? null) : null),
    cancelled: run.state === 'cancelled',
    working: run.state === 'running',
    cost,
    startedAt: run.started_at,
  }
}

/**
 * Add one event to a run already in hand.
 *
 * The panel keeps runs as they came from the backend and appends what arrives
 * on the event channel, so a live run and a replayed one are the same shape.
 */
export function withEvent(run: Run, event: RunEvent): Run {
  const events = [...run.events, event]
  const state =
    event.kind === 'finished'
      ? 'done'
      : event.kind === 'failed'
        ? 'failed'
        : event.kind === 'stopped'
          ? 'cancelled'
          : run.state

  return { ...run, events, state }
}

/** Runs of a chat, oldest first — the order a conversation is read in. */
export function inOrder(runs: Run[]): Run[] {
  return [...runs].sort((left, right) => {
    if (left.started_at !== right.started_at) {
      return left.started_at < right.started_at ? -1 : 1
    }
    // Runs started inside the same second still need a stable order.
    return left.id < right.id ? -1 : 1
  })
}
