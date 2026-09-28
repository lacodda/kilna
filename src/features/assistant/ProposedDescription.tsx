import { useTranslation } from 'react-i18next'
import type { Applied } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { useApplyProposal } from '@/lib/useApplyProposal'
import { ProposalCard } from '@/features/assistant/ProposalCard'

interface Props {
  messageId: string
  /** Set once somebody applied it; read from the message. */
  applied: Applied | null
  /** Set once somebody turned it down; read from the message. */
  dismissed: boolean
}

/**
 * A style brick's description, read off its references, next to the button
 * that keeps it.
 *
 * The answer above is the description, word for word, as a reply's answer is
 * the reply. Kept, it is written onto the brick the task was started from -
 * the same edit the dictionary makes, so undo takes it back - and the brick
 * leaves its draft. Until v0.77 the answer could only be copied by hand: the
 * command that kept it had no button.
 */
export function ProposedDescription({ messageId, applied, dismissed }: Props) {
  const { t } = useTranslation()

  const keep = useApplyProposal({
    messageId,
    message: t('assistant.descriptionAdded'),
    refresh: [keys.styles],
  })

  return (
    <ProposalCard
      messageId={messageId}
      title={t('assistant.proposedDescription')}
      applied={applied}
      dismissed={dismissed}
      applyLabel={t('assistant.keepDescription')}
      onApply={() => {
        keep.mutate(undefined)
      }}
      applying={keep.isPending}
      appliedLabel={t('assistant.descriptionKept')}
    />
  )
}
