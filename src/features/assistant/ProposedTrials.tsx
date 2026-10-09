import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import type { Applied, TrialsProposal } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { useApplyProposal } from '@/lib/useApplyProposal'
import { Button } from '@/components/ui/button'
import { ProposalCard } from '@/features/assistant/ProposalCard'

interface Props {
  messageId: string
  proposal: TrialsProposal
  /** Set once somebody applied it; read from the message. */
  applied: Applied | null
  /** Set once somebody turned it down; read from the message. */
  dismissed: boolean
}

/**
 * Trials for an experiment's board an agent proposed (v0.95, ADR 0061), next
 * to the one button that puts them on the board. The message above reads each
 * trial; what the agent named and the workspace does not have is said here.
 * Trials asked for from the board itself land there on their own, and their
 * card only says so.
 */
export function ProposedTrials({ messageId, proposal, applied, dismissed }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const apply = useApplyProposal({
    messageId,
    message: t('trials.putDone', { count: proposal.trials.length }),
    refresh: [keys.trialBoard(proposal.work_id)],
  })

  return (
    <ProposalCard
      messageId={messageId}
      title={t('trials.proposed', { count: proposal.trials.length })}
      warnings={(proposal.dropped ?? []).map((one) =>
        t('trials.dropped', {
          trial: one.trial,
          part: t(`trials.part.${one.part}`, { defaultValue: one.part }),
          value: one.value,
        }),
      )}
      applied={applied}
      dismissed={dismissed}
      applyLabel={t('trials.putProposed')}
      onApply={() => {
        apply.mutate(undefined)
      }}
      applying={apply.isPending}
      appliedLabel={t('trials.putMark')}
      afterApplied={
        <Button
          size="xs"
          variant="ghost"
          onClick={() => {
            void navigate(`/works/${proposal.work_id}/trials`)
          }}
        >
          {t('trials.openBoard')}
        </Button>
      }
    />
  )
}
