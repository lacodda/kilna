import { useQuery } from '@tanstack/react-query'
import type { ScoreProposal } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { ProposedScore } from '@/features/assistant/ProposedScore'

interface Props {
  workId: string
}

/**
 * The scores proposed for this work that wait for an answer (v0.108): from a
 * plugin on this tab, from an agent, from an action in a chat. A verdict waits
 * where verdicts are given. Kept, it is an ordinary score in the list beside
 * it; turned down, it stops waiting. Either way the chat it came through still
 * shows what was said.
 */
export function WaitingScores({ workId }: Props) {
  const waiting = useQuery(queries.waitingScores(workId)).data ?? []
  if (waiting.length === 0) return null

  return (
    <div className="flex w-full flex-col gap-2">
      {waiting.flatMap((score) =>
        score.proposal.kind === 'score'
          ? [
              <ProposedScore
                key={score.message_id}
                workId={workId}
                messageId={score.message_id}
                proposal={score.proposal as ScoreProposal}
                client={score.client}
                applied={null}
                dismissed={false}
              />,
            ]
          : [],
      )}
    </div>
  )
}
