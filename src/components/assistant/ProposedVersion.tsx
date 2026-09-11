import { useTranslation } from 'react-i18next'
import type { Applied, VersionProposal } from '@/lib/api'
import { keys } from '@/lib/query'
import { useApplyProposal } from '@/lib/useApplyProposal'
import { useVocabulary } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { AppliedMark } from '@/components/assistant/AppliedMark'

interface Props {
  workId: string
  messageId: string
  proposal: VersionProposal
  /** Set once somebody applied it; read from the message. */
  applied: Applied | null
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
export function ProposedVersion({ workId, messageId, proposal, applied, onChoose }: Props) {
  const { t } = useTranslation()
  const roles = useVocabulary(workId).version_roles
  const role = roles.find((r) => r.key === proposal.role)?.label ?? proposal.role

  const apply = useApplyProposal({
    messageId,
    message: t('assistant.inserted'),
    refresh: [keys.versions(workId), keys.work(workId), keys.works],
  })

  return (
    <div className="mx-3 flex flex-wrap items-center gap-2 rounded-xl border border-line px-3 py-2 text-sm">
      <span className="text-dim">
        {t('assistant.proposedVersion')}
        {' · '}
        <b className="font-semibold text-text">{role}</b>
        {proposal.label !== undefined && proposal.label !== '' && (
          <>
            {' · '}
            {proposal.label}
          </>
        )}
      </span>
      <span className="ml-auto flex items-center gap-1.5">
        {applied !== null ? (
          <AppliedMark applied={applied} label={t('assistant.versionKept')} />
        ) : (
          <>
            <Button size="sm" disabled={apply.isPending} onClick={onChoose}>
              {t('assistant.insertChoose')}
            </Button>
            <Button
              size="sm"
              variant="primary"
              disabled={apply.isPending}
              onClick={() => {
                apply.mutate(undefined)
              }}
            >
              {t('assistant.insert')}
            </Button>
          </>
        )}
      </span>
    </div>
  )
}
