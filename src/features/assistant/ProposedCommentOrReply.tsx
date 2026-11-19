import { useState } from 'react'
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
  // Three ways to answer (v0.162): the one chosen is what is kept.
  const variants = proposal.kind === 'reply' ? (proposal.variants ?? []) : []
  const [chosen, setChosen] = useState(0)

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
        const picked = variants[chosen]
        keep.mutate(picked === undefined ? undefined : { reply: picked.reply })
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
      {proposal.kind === 'reply' && variants.length > 0 && (
        <div className="flex flex-col gap-2 text-xs">
          {proposal.translation !== undefined && (
            <p className="text-dim">
              <span className="font-semibold text-text">{t('replies.comment')}</span>{' '}
              {proposal.translation}
            </p>
          )}
          {proposal.context !== undefined && (
            <p className="text-dim">
              <span className="font-semibold text-text">{t('replies.context')}</span>{' '}
              {proposal.context}
            </p>
          )}
          <div role="radiogroup" aria-label={t('replies.choose')} className="flex flex-col gap-1">
            {variants.map((variant, index) => (
              <Button
                key={`${String(index)}:${variant.reply}`}
                variant={chosen === index ? 'soft' : 'ghost'}
                role="radio"
                aria-checked={chosen === index}
                disabled={applied !== null || dismissed}
                onClick={() => setChosen(index)}
                className="h-auto flex-col items-start gap-0.5 py-1.5 text-left whitespace-normal"
              >
                <span className="text-sm text-text">{variant.reply}</span>
                {variant.translation !== undefined && (
                  <span className="text-xs font-normal text-dim">{variant.translation}</span>
                )}
              </Button>
            ))}
          </div>
        </div>
      )}
    </ProposalCard>
  )
}
