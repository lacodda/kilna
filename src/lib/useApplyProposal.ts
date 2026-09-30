import { useQueryClient } from '@tanstack/react-query'
import { applyProposal } from '@/lib/api/assistant'
import type { Applied, ProposalOverrides } from '@/lib/api/types'
import { announceEdited } from '@/lib/edited'
import { keys } from '@/lib/query/keys'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'

interface Options {
  /** The message carrying the proposal. */
  messageId: string
  /** What to say when it lands, in the person's words. */
  message: string
  /** The query areas the write disturbs; the transcript and the journal are
   * always among them — the mark lives on the message, the line in the feed. */
  refresh: readonly (readonly unknown[])[]
  onApplied?: (applied: Applied) => void
}

/** How many operations applying wrote: the work, each version, the score, each note, the
 * fields, each scene written, each scene a replaced board sent to the trash, a style's
 * description, and each card, fact and relation of the canon. */
export function writesOf(applied: Applied): number {
  return (
    (applied.created_work === true ? 1 : 0) +
    (applied.versions?.length ?? 0) +
    (applied.score !== undefined ? 1 : 0) +
    (applied.notes?.length ?? 0) +
    (applied.created_work !== true && (applied.fields?.length ?? 0) > 0 ? 1 : 0) +
    (applied.scenes?.length ?? 0) +
    (applied.removed_scenes?.length ?? 0) +
    (applied.style_brick !== undefined ? 1 : 0) +
    (applied.cards?.length ?? 0) +
    (applied.facts?.length ?? 0) +
    (applied.relations?.length ?? 0)
  )
}

/**
 * Apply what a message proposes — one command for every kind.
 *
 * The proposal is written by the backend, which records the same operations
 * a hand would and stamps the message; this hook only refreshes what moved
 * and says so. Before v0.57.1 each proposal component called the command a
 * hand uses and remembered "applied" in its own state, which the next fetch
 * of the transcript forgot.
 */
export function useApplyProposal({ messageId, message, refresh, onApplied }: Options) {
  const client = useQueryClient()

  const disturbed = [...refresh, keys.transcripts]
  return useAppMutation({
    mutationFn: (overrides?: ProposalOverrides) => applyProposal(messageId, overrides),
    refresh: disturbed,
    onSuccess: (applied) => {
      // The undo offer takes back the last operation, and only that. A single
      // version, score or note is one operation; a package is several, and
      // an offer that would remove the last note and leave the work behind
      // is worse than no offer. Ctrl+Z still walks them back one at a time.
      if (writesOf(applied) === 1) announceEdited({ client, message, refresh: disturbed })
      else say.ok(message)
      onApplied?.(applied)
    },
    onError: (cause) => {
      say.failed(cause)
      // "Already applied" from another window is the likely reason; the
      // transcript then shows the mark this one did not have yet.
      void client.invalidateQueries({ queryKey: keys.transcripts })
    },
  })
}
