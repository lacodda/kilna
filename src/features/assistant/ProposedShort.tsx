import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import type { Applied, ShortProposal } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { refresh } from '@/lib/query/refresh'
import { worn } from '@/lib/shorts/style'
import { useApplyProposal } from '@/lib/useApplyProposal'
import { Button } from '@/components/ui/button'
import { ProposalCard } from '@/features/assistant/ProposalCard'
import { ShortFrame } from '@/features/shorts/ShortFrame'

interface Props {
  messageId: string
  proposal: ShortProposal
  /** Set once somebody applied it; read from the message. */
  applied: Applied | null
  /** Set once somebody turned it down; read from the message. */
  dismissed: boolean
}

/**
 * What an agent proposed for the space's shorts (v0.168): the style whole -
 * drawn as one frame, in the variant it names or as it is - or the variant
 * one short is to wear. Taken, the style stands in place of the space's as
 * one edit of the profile, and the variant is written into the short's
 * fields; undo takes either back.
 */
export function ProposedShort({ messageId, proposal, applied, dismissed }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const style = proposal.style ?? null
  const variant = proposal.variant ?? null

  const take = useApplyProposal({
    messageId,
    message: t('shorts.proposalTaken'),
    refresh: [keys.workspace, keys.cuts, ...refresh.work, keys.pendingProposals],
  })

  return (
    <ProposalCard
      messageId={messageId}
      title={style === null ? t('shorts.proposedVariant') : t('shorts.proposedStyle')}
      applied={applied}
      dismissed={dismissed}
      applyLabel={t('shorts.takeProposal')}
      applying={take.isPending}
      onApply={() => take.mutate(undefined)}
      appliedLabel={t('shorts.proposalTakenMark')}
      afterApplied={
        style === null ? undefined : (
          <Button size="xs" variant="ghost" onClick={() => void navigate('/settings/shorts')}>
            {t('shorts.openStyle')}
          </Button>
        )
      }
    >
      {style !== null && (
        <ShortFrame
          look={worn(
            style,
            style.variants.find((entry) => entry.key === variant) ?? style.variants[0],
          )}
          title={t('shorts.sampleTitle')}
          className="w-40"
        />
      )}
    </ProposalCard>
  )
}
