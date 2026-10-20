import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import type { Applied, CardsProposal } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { useApplyProposal } from '@/lib/useApplyProposal'
import { Button } from '@/components/ui/button'
import { ProposalCard } from '@/features/assistant/ProposalCard'

interface Props {
  messageId: string
  proposal: CardsProposal
  /** Set once somebody applied it; read from the message. */
  applied: Applied | null
  /** Set once somebody turned it down; read from the message. */
  dismissed: boolean
}

/**
 * Cards for a board of a conception an agent proposed (v0.99, ADR 0065), next
 * to the one button that puts them on the board. The message above reads each
 * card; what the agent named and the board does not have is said here. Cards
 * asked for from the board itself land there on their own, and their card
 * only says so.
 */
export function ProposedCards({ messageId, proposal, applied, dismissed }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const apply = useApplyProposal({
    messageId,
    message: t('board.putDone', { count: proposal.cards.length }),
    refresh: [keys.boards(proposal.work_id)],
  })

  return (
    <ProposalCard
      messageId={messageId}
      title={t('board.proposed', { count: proposal.cards.length })}
      warnings={(proposal.dropped ?? []).map((one) =>
        t('board.dropped', {
          card: one.card,
          part: t(`board.part.${one.part}`, { defaultValue: one.part }),
          value: one.value,
        }),
      )}
      applied={applied}
      dismissed={dismissed}
      applyLabel={t('board.putProposed')}
      onApply={() => {
        apply.mutate(undefined)
      }}
      applying={apply.isPending}
      appliedLabel={t('board.putMark')}
      afterApplied={
        <Button
          size="xs"
          variant="ghost"
          onClick={() => {
            void navigate(`/works/${proposal.work_id}/conception`)
          }}
        >
          {t('board.openBoard')}
        </Button>
      }
    />
  )
}
