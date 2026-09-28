import { useTranslation } from 'react-i18next'
import type { Applied, VersionProposal } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { useApplyProposal } from '@/lib/useApplyProposal'
import { say, useVocabulary } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { ProposalCard } from '@/features/assistant/ProposalCard'

interface Props {
  workId: string
  messageId: string
  proposal: VersionProposal
  /** Set once somebody applied it; read from the message. */
  applied: Applied | null
  /** Set once somebody turned it down; read from the message. */
  dismissed: boolean
  /** Opens the dialog for a person who wants another role, a name, or to
   * make it current on the way in. */
  onChoose: () => void
}

/**
 * A version an agent proposed, next to the button that keeps it.
 *
 * One click keeps it under the role the agent named, not current — an answer
 * worth keeping is not yet an answer worth standing behind, the dialog's own
 * default. The dialog is one click further for the person who wants to
 * change any of that first.
 */
export function ProposedVersion({
  workId,
  messageId,
  proposal,
  applied,
  dismissed,
  onChoose,
}: Props) {
  const { t } = useTranslation()
  const roles = useVocabulary(workId).version_roles
  const roleLabel = roles.find((r) => r.key === proposal.role)?.label
  const role = roleLabel === undefined ? proposal.role : say(roleLabel)

  const apply = useApplyProposal({
    messageId,
    message: t('assistant.inserted'),
    refresh: [keys.versions(workId), keys.work(workId), keys.works],
  })

  return (
    <ProposalCard
      messageId={messageId}
      title={t('assistant.proposedVersion')}
      applied={applied}
      dismissed={dismissed}
      applyLabel={t('assistant.insert')}
      onApply={() => {
        apply.mutate(undefined)
      }}
      applying={apply.isPending}
      more={
        <Button size="sm" variant="ghost" disabled={apply.isPending} onClick={onChoose}>
          {t('assistant.insertChoose')}
        </Button>
      }
      appliedLabel={t('assistant.versionKept')}
    >
      <p>
        <b className="font-semibold text-text">{role}</b>
        {proposal.label !== undefined && proposal.label !== '' && (
          <>
            {' · '}
            {proposal.label}
          </>
        )}
      </p>
    </ProposalCard>
  )
}
