import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { Applied, Proposal } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useApplyProposal } from '@/lib/useApplyProposal'
import { say as sayLabel } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { ProposalCard } from '@/features/assistant/ProposalCard'

type ReleaseProposalKind = Extract<Proposal, { kind: 'release' }>

interface Props {
  messageId: string
  proposal: ReleaseProposalKind
  applied: Applied | null
  dismissed: boolean
  /** The work the chat is on: where "Open the release" goes. */
  workId?: string
}

/**
 * What a release should go out under, as an answer proposed it (v0.86): each
 * field with the text proposed for it, and the button that takes what is left
 * of it.
 *
 * The release's own action has already filled the fields that were empty when
 * the answer came; taking here writes the rest - the backend knows which
 * those are. Choosing field by field is the release's own business, under
 * its fields on the Releases tab, where each is read beside what it replaces.
 */
export function ProposedRelease({ messageId, proposal, applied, dismissed, workId }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()

  // The fields' names, in the profile's words and order: the proposal holds
  // only their keys.
  const fields = useQuery(queries.releaseFields(proposal.release_id))
  const labelOf = (key: string) => {
    const field = fields.data?.find((one) => one.key === key)
    return field === undefined ? key : sayLabel(field.label)
  }
  const order = (fields.data ?? []).map((field) => field.key)
  const proposed = Object.entries(proposal.fields)
    .filter((entry): entry is [string, string] => typeof entry[1] === 'string')
    .sort(([a], [b]) => {
      const left = order.indexOf(a)
      const right = order.indexOf(b)
      return (left === -1 ? order.length : left) - (right === -1 ? order.length : right)
    })

  const take = useApplyProposal({
    messageId,
    message: t('releases.proposals.taken'),
    refresh: [
      ...refresh.release,
      keys.releaseFields(proposal.release_id),
      keys.releaseProposalsFor(proposal.release_id),
      keys.pendingProposals,
    ],
  })

  const unknown = proposal.unknown ?? []

  return (
    <ProposalCard
      messageId={messageId}
      title={t('releases.proposals.inChat')}
      figure={String(proposed.length)}
      applied={applied}
      dismissed={dismissed}
      applyLabel={t('releases.proposals.takeAll')}
      onApply={() => take.mutate(undefined)}
      applying={take.isPending}
      appliedLabel={t('releases.proposals.takenMark')}
      warnings={
        unknown.length === 0 ? [] : [t('releases.proposals.unknown', { keys: unknown.join(', ') })]
      }
      afterApplied={
        workId === undefined ? undefined : (
          <Button size="sm" variant="link" onClick={() => void navigate(`/works/${workId}`)}>
            {t('releases.metaStatus.open')}
          </Button>
        )
      }
    >
      <dl className="grid grid-cols-[auto_1fr] gap-x-2.5 gap-y-1">
        {proposed.map(([key, value]) => (
          <div key={key} className="contents">
            <dt className="text-faint">{labelOf(key)}</dt>
            <dd className="line-clamp-3 whitespace-pre-wrap text-text selectable">{value}</dd>
          </div>
        ))}
      </dl>
    </ProposalCard>
  )
}
