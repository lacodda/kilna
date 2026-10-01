import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import type { Applied, CoverIdeasProposal } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { useApplyProposal } from '@/lib/useApplyProposal'
import { Button } from '@/components/ui/button'
import { ProposalCard } from '@/features/assistant/ProposalCard'

interface Props {
  messageId: string
  proposal: CoverIdeasProposal
  /** Set once somebody applied it; read from the message. */
  applied: Applied | null
  /** Set once somebody turned it down; read from the message. */
  dismissed: boolean
}

/**
 * Ideas for a cover an agent proposed (v0.89, ADR 0050), next to the one
 * button that puts them on the board. The message above reads each idea; what
 * the agent named and the workspace does not have is said here, since it was
 * left out of its idea. Ideas asked for from the board itself land there on
 * their own, and their card only says so.
 */
export function ProposedCoverIdeas({ messageId, proposal, applied, dismissed }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const apply = useApplyProposal({
    messageId,
    message: t('ideas.putDone', { count: proposal.ideas.length }),
    refresh: [keys.coverBoard(proposal.work_id)],
  })

  return (
    <ProposalCard
      messageId={messageId}
      title={t('ideas.proposed', { count: proposal.ideas.length })}
      warnings={(proposal.dropped ?? []).map((one) =>
        t('ideas.dropped', {
          idea: one.idea,
          part: t(`ideas.part.${one.part}`, { defaultValue: one.part }),
          value: one.value,
        }),
      )}
      applied={applied}
      dismissed={dismissed}
      applyLabel={t('ideas.putProposed')}
      onApply={() => {
        apply.mutate(undefined)
      }}
      applying={apply.isPending}
      appliedLabel={t('ideas.putMark')}
      afterApplied={
        <Button
          size="xs"
          variant="ghost"
          onClick={() => {
            void navigate(`/works/${proposal.work_id}/cover`)
          }}
        >
          {t('ideas.openBoard')}
        </Button>
      }
    />
  )
}
