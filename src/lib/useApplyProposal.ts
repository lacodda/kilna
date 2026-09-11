import { useMutation, useQueryClient } from '@tanstack/react-query'
import { applyProposal, type Applied, type ProposalOverrides } from '@/lib/api'
import { announceEdited } from '@/lib/edited'
import { keys } from '@/lib/query'
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

  return useMutation({
    mutationFn: (overrides?: ProposalOverrides) => applyProposal(messageId, overrides),
    onSuccess: (applied) => {
      const disturbed = [...refresh, keys.transcripts, keys.journal]
      for (const key of disturbed) void client.invalidateQueries({ queryKey: key })
      announceEdited({ client, message, refresh: disturbed })
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
