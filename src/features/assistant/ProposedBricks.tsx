import { useState } from 'react'
import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import type { Applied, BricksProposal } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { refresh } from '@/lib/query/refresh'
import { useApplyProposal } from '@/lib/useApplyProposal'
import { Button } from '@/components/ui/button'
import { ProposalCard } from '@/features/assistant/ProposalCard'
import { BricksPackageView, brickItems } from '@/features/styles/BricksPackageView'

interface Props {
  messageId: string
  proposal: BricksProposal
  /** Set once somebody applied it; read from the message. */
  applied: Applied | null
  /** Set once somebody turned it down; read from the message. */
  dismissed: boolean
}

/**
 * Bricks for the dictionary that "Explain" answered with (v0.94): each phrase
 * a text says and the dictionary did not know, with its type and what it
 * means. Kept brick by brick - each one an ordinary new style, which undo
 * takes back - and the way on leads to the dictionary.
 */
export function ProposedBricks({ messageId, proposal, applied, dismissed }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const pack = proposal.package
  const all = brickItems(pack)
  const [chosen, setChosen] = useState<string[]>(all)

  const keep = useApplyProposal({
    messageId,
    message: t('phrases.keptFromChat'),
    refresh: [...refresh.style, keys.pendingProposals],
  })

  const first = applied?.style_bricks?.[0]
  const answered = applied !== null || dismissed

  return (
    <ProposalCard
      messageId={messageId}
      title={t('assistant.proposedBricks')}
      figure={answered ? undefined : String(chosen.length)}
      applied={applied}
      dismissed={dismissed}
      applyLabel={
        chosen.length === all.length
          ? t('phrases.keepAll')
          : t('phrases.keep', { n: chosen.length })
      }
      applying={keep.isPending || chosen.length === 0}
      onApply={() => keep.mutate(chosen.length === all.length ? undefined : { items: chosen })}
      appliedLabel={t('phrases.keptMark', { count: applied?.style_bricks?.length ?? 0 })}
      afterApplied={
        first === undefined ? undefined : (
          <Button size="xs" variant="ghost" onClick={() => void navigate(`/styles/${first}`)}>
            {t('phrases.openKept')}
          </Button>
        )
      }
    >
      <BricksPackageView pack={pack} chosen={chosen} onChosen={setChosen} answered={answered} />
    </ProposalCard>
  )
}
