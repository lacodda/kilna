import { useTranslation } from 'react-i18next'
import type { Applied } from '@/lib/api'
import { keys } from '@/lib/query'
import { useApplyProposal } from '@/lib/useApplyProposal'
import { Button } from '@/components/ui/button'
import { AppliedMark } from '@/components/assistant/AppliedMark'

interface Props {
  messageId: string
  /** Set once somebody applied it; read from the message. */
  applied: Applied | null
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
export function ProposedDescription({ messageId, applied }: Props) {
  const { t } = useTranslation()

  const keep = useApplyProposal({
    messageId,
    message: t('assistant.descriptionAdded'),
    refresh: [keys.styles],
  })

  return (
    <div className="mx-3 flex flex-wrap items-center gap-2 rounded-xl border border-line px-3 py-2 text-sm">
      <span className="text-dim">{t('assistant.proposedDescription')}</span>
      <span className="ml-auto">
        {applied !== null ? (
          <AppliedMark applied={applied} label={t('assistant.descriptionKept')} />
        ) : (
          <Button
            size="sm"
            variant="primary"
            disabled={keep.isPending}
            onClick={() => {
              keep.mutate(undefined)
            }}
          >
            {t('assistant.keepDescription')}
          </Button>
        )}
      </span>
    </div>
  )
}
