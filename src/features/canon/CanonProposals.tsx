import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { dismissProposal } from '@/lib/api/assistant'
import type { CanonProposal } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { useApplyProposal } from '@/lib/useApplyProposal'
import { useAssistant } from '@/lib/useAssistant'
import { Button } from '@/components/ui/button'
import { Dialog } from '@/components/AppDialog'
import { CanonPackageView, itemsOf } from '@/features/canon/CanonPackageView'

/** What a proposal disturbs once kept: the canon, and the proposals' lists. */
const KEPT = [...refresh.canon, keys.canonProposals, keys.pendingProposals]

/**
 * What the assistant proposes for the open card, waiting to be kept: a box
 * per proposal with what it holds, "Review" to read it item by item against
 * the canon, and "Apply all".
 */
export function CanonProposals({ proposals }: { proposals: CanonProposal[] }) {
  return (
    <div className="flex flex-col gap-2">
      {proposals.map((proposal) => (
        <ProposalBox key={proposal.message_id} proposal={proposal} />
      ))}
    </div>
  )
}

function ProposalBox({ proposal }: { proposal: CanonProposal }) {
  const { t } = useTranslation()
  const assistant = useAssistant()
  const [reviewing, setReviewing] = useState(false)
  const keep = useApplyProposal({
    messageId: proposal.message_id,
    message: t('canon.kept'),
    refresh: KEPT,
  })
  const inner = proposal.proposal
  const counts =
    inner.kind === 'canon'
      ? t('canon.proposalHolds', {
          cards: inner.package.cards.length,
          facts: inner.package.facts.length,
          relations: inner.package.links.length,
          pictures: inner.package.pictures.length,
          descriptions: inner.package.descriptions.length,
        })
      : t('canon.proposalDescribes')

  return (
    <section className="flex flex-col gap-1.5 rounded-lg border border-info bg-info-soft px-2.5 py-2 text-xs">
      <b className="font-semibold">{t('canon.assistantProposes')}</b>
      <span className="text-dim">{counts}</span>
      {proposal.chat_title !== null && <span className="text-faint">{proposal.chat_title}</span>}
      <div className="flex flex-wrap gap-1.5">
        <Button size="xs" variant="soft" onClick={() => setReviewing(true)}>
          {t('canon.review')}
        </Button>
        <Button
          size="xs"
          variant="ghost"
          disabled={keep.isPending}
          onClick={() => keep.mutate(undefined)}
        >
          {t('canon.applyAll')}
        </Button>
        <Button size="xs" variant="link" onClick={() => assistant.open(proposal.chat_id)}>
          {t('canon.openChat')}
        </Button>
      </div>
      {reviewing && <ReviewDialog proposal={proposal} open onOpenChange={setReviewing} />}
    </section>
  )
}

/** A proposal read item by item, and kept in part or whole. */
function ReviewDialog({
  proposal,
  open,
  onOpenChange,
}: {
  proposal: CanonProposal
  open: boolean
  onOpenChange: (open: boolean) => void
}) {
  const { t } = useTranslation()
  const inner = proposal.proposal
  const all = inner.kind === 'canon' ? itemsOf(inner.package) : []
  const [chosen, setChosen] = useState<string[]>(all)
  const keep = useApplyProposal({
    messageId: proposal.message_id,
    message: t('canon.kept'),
    refresh: KEPT,
    onApplied: () => onOpenChange(false),
  })
  const dismiss = useAppMutation({
    mutationFn: () => dismissProposal(proposal.message_id),
    failure: 'assistant.dismissFailed',
    refresh: [keys.canonProposals, keys.pendingProposals, keys.transcripts],
    onSuccess: () => onOpenChange(false),
  })

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={t('canon.reviewTitle')}
      description={proposal.chat_title ?? undefined}
      size="lg"
      dirty
      aside={
        <Button variant="ghost" disabled={dismiss.isPending} onClick={() => dismiss.mutate()}>
          {t('assistant.dismiss')}
        </Button>
      }
      footer={
        <Button
          variant="primary"
          disabled={keep.isPending || (inner.kind === 'canon' && chosen.length === 0)}
          onClick={() =>
            keep.mutate(
              inner.kind === 'canon' && chosen.length !== all.length
                ? { items: chosen }
                : undefined,
            )
          }
        >
          {inner.kind === 'canon'
            ? t('canon.keepChosen', { count: chosen.length })
            : t('canon.keepDescription')}
        </Button>
      }
    >
      {inner.kind === 'canon' ? (
        <CanonPackageView
          messageId={proposal.message_id}
          pack={inner.package}
          chosen={chosen}
          onChosen={setChosen}
          answered={false}
        />
      ) : (
        <p className="rounded-lg border border-line bg-bg p-2.5 font-mono text-xs leading-relaxed text-dim">
          {proposal.body}
        </p>
      )}
    </Dialog>
  )
}
