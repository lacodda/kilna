import { useTranslation } from 'react-i18next'
import type { Applied, MediaProposal } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { useApplyProposal } from '@/lib/useApplyProposal'
import { ProposalCard } from '@/features/assistant/ProposalCard'

interface Props {
  messageId: string
  proposal: MediaProposal
  applied: Applied | null
  dismissed: boolean
}

/** The last step of a path, whichever slash it is written with. */
const nameOf = (path: string) => path.split(/[\\/]/).pop() ?? path

/**
 * Files an assistant made beside kilna, each for a work and as what
 * (v0.156, FR-21 21.1 p.4): read before anything moves, and moved into
 * their works' folders on one press - each checked, renamed after its work
 * and given its role.
 */
export function ProposedMedia({ messageId, proposal, applied, dismissed }: Props) {
  const { t } = useTranslation()
  const count = proposal.items.length
  const apply = useApplyProposal({
    messageId,
    message: t('receiver.proposalApplied', { count }),
    refresh: [keys.works, keys.catalogue, keys.covers, keys.folders],
  })

  return (
    <ProposalCard
      messageId={messageId}
      title={t('receiver.proposed', { count })}
      applied={applied}
      dismissed={dismissed}
      applyLabel={t('receiver.applyAll', { count })}
      onApply={() => {
        apply.mutate(undefined)
      }}
      applying={apply.isPending}
      appliedLabel={t('receiver.proposalAppliedMark')}
    >
      <ul className="flex flex-col gap-0.5 text-xs">
        {proposal.items.map((item, index) => (
          <li
            key={`${item.path}:${String(index)}`}
            className="flex flex-wrap items-baseline gap-1.5"
          >
            <b className="font-semibold">{item.work_title}</b>
            <span className="text-dim">{t(`receiver.role.${item.role}`)}</span>
            <span className="min-w-0 truncate text-faint" title={item.path}>
              {nameOf(item.path)}
            </span>
          </li>
        ))}
      </ul>
    </ProposalCard>
  )
}
