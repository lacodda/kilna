import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import type { Applied, CommentProposal, ReplyProposal } from '@/lib/api/types'
import { refresh } from '@/lib/query/refresh'
import { useApplyProposal } from '@/lib/useApplyProposal'
import { Button } from '@/components/ui/button'
import { ProposalCard } from '@/features/assistant/ProposalCard'

interface Props {
  messageId: string
  proposal: CommentProposal | ReplyProposal
  /** Set once somebody applied it; read from the message. */
  applied: Applied | null
  /** Set once somebody turned it down; read from the message. */
  dismissed: boolean
}

/**
 * A drafted reply, or a comment read off a screenshot, as the chat shows it.
 *
 * Both are at home on the comments screen, where a reply stands under its
 * comment and a reading's every field can be corrected before it is kept -
 * a misread name, a wrong day. Until v0.81 the chat only pointed there. It
 * keeps the way there as the second button, and now also takes the proposal
 * as it stands in one click, or turns it down, like every other card: the
 * answer above is the reply word for word, and the reading's fields are the
 * line under the caption.
 */
export function ProposedCommentOrReply({ messageId, proposal, applied, dismissed }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const reply = proposal.kind === 'reply'

  const keep = useApplyProposal({
    messageId,
    message: t('assistant.commentKept'),
    refresh: refresh.keptComment,
  })

  // A reply goes to its comment; a reading has no comment yet, only the inbox.
  const where = reply ? `/comments/${proposal.comment_id}` : '/comments'
  const read =
    proposal.kind === 'comment'
      ? [proposal.author, proposal.channel, proposal.commented_on, proposal.about].filter(
          (part): part is string => part !== undefined && part !== '',
        )
      : []

  return (
    <ProposalCard
      messageId={messageId}
      title={t(reply ? 'assistant.proposed.reply' : 'assistant.proposed.comment')}
      applied={applied}
      dismissed={dismissed}
      applyLabel={t(reply ? 'assistant.keepReply' : 'assistant.keepComment')}
      onApply={() => {
        keep.mutate(undefined)
      }}
      applying={keep.isPending}
      more={
        <Button
          size="sm"
          variant="ghost"
          disabled={keep.isPending}
          onClick={() => {
            void navigate(where)
          }}
        >
          {t('assistant.openComments')}
        </Button>
      }
      appliedLabel={t('assistant.commentKept')}
    >
      {read.length > 0 && <p>{read.join(' · ')}</p>}
    </ProposalCard>
  )
}
