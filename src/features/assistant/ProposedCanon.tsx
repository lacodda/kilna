import { useState } from 'react'
import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import type { Applied, CanonProposalKind } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { refresh } from '@/lib/query/refresh'
import { useApplyProposal } from '@/lib/useApplyProposal'
import { Button } from '@/components/ui/button'
import { ProposalCard } from '@/features/assistant/ProposalCard'
import { CanonPackageView, itemsOf } from '@/features/canon/CanonPackageView'

interface Props {
  messageId: string
  proposal: CanonProposalKind
  applied: Applied | null
  dismissed: boolean
}

/**
 * Cards, facts and relations for the canon, item by item under the answer
 * that proposed them, each with a box to keep it or leave it out - and the
 * facts it would contradict shown beside it before anything is written
 * (ADR 0043).
 */
export function ProposedCanon({ messageId, proposal, applied, dismissed }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const pack = proposal.package
  const all = itemsOf(pack)
  const [chosen, setChosen] = useState<string[]>(all)

  const keep = useApplyProposal({
    messageId,
    message: t('canon.kept'),
    refresh: [...refresh.canon, keys.canonProposals, keys.pendingProposals],
  })
  const firstCard = applied?.cards?.[0] ?? applied?.facts?.[0] ?? null

  return (
    <ProposalCard
      messageId={messageId}
      title={t('assistant.proposedCanon')}
      figure={String(chosen.length)}
      applied={applied}
      dismissed={dismissed}
      applyLabel={
        chosen.length === all.length
          ? t('canon.applyAll')
          : t('canon.keepChosen', { count: chosen.length })
      }
      applying={keep.isPending || chosen.length === 0}
      onApply={() => keep.mutate(chosen.length === all.length ? undefined : { items: chosen })}
      appliedLabel={t('canon.keptMark')}
      afterApplied={
        firstCard !== null && applied?.cards !== undefined && applied.cards.length > 0 ? (
          <Button
            size="sm"
            variant="link"
            onClick={() => void navigate(`/canon/${applied.cards?.[0] ?? ''}`)}
          >
            {t('canon.openCard')}
          </Button>
        ) : undefined
      }
    >
      <CanonPackageView
        messageId={messageId}
        pack={pack}
        chosen={chosen}
        onChosen={setChosen}
        answered={applied !== null || dismissed}
      />
    </ProposalCard>
  )
}

/** A card's description for a picture generator, next to the button that
 *  keeps it. The answer above is the description, word for word. */
export function ProposedCardPrompt({
  messageId,
  noteId,
  applied,
  dismissed,
}: {
  messageId: string
  noteId: string
  applied: Applied | null
  dismissed: boolean
}) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const keep = useApplyProposal({
    messageId,
    message: t('canon.descriptionKept'),
    refresh: [...refresh.canon, keys.canonProposals, keys.pendingProposals],
  })
  return (
    <ProposalCard
      messageId={messageId}
      title={t('assistant.proposedCardPrompt')}
      applied={applied}
      dismissed={dismissed}
      applyLabel={t('canon.keepDescription')}
      onApply={() => keep.mutate(undefined)}
      applying={keep.isPending}
      appliedLabel={t('canon.descriptionKeptMark')}
      afterApplied={
        <Button size="sm" variant="link" onClick={() => void navigate(`/canon/${noteId}`)}>
          {t('canon.openCard')}
        </Button>
      }
    />
  )
}
