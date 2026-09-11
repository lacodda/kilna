import { useTranslation } from 'react-i18next'
import type { Applied, NoteProposal } from '@/lib/api'
import { keys } from '@/lib/query'
import { useApplyProposal } from '@/lib/useApplyProposal'
import { Button } from '@/components/ui/button'
import { AppliedMark } from '@/components/assistant/AppliedMark'

interface Props {
  messageId: string
  proposal: NoteProposal
  /** Set once somebody applied it; read from the message. */
  applied: Applied | null
}

/**
 * A note an agent proposed, next to the button that keeps it.
 *
 * Same bargain as a proposed score: the text is readable before it is
 * written, and a person writes it. Kept, it is an ordinary note — the same
 * table, no mark saying a machine suggested it — on the chat's work, or on
 * nothing when the chat is about nothing. The toast offers to take it back,
 * since adding a note is one of the things undo covers.
 */
export function ProposedNote({ messageId, proposal, applied }: Props) {
  const { t } = useTranslation()

  const keep = useApplyProposal({
    messageId,
    message: t('assistant.noteAdded'),
    refresh: [keys.notes],
  })

  return (
    <div className="mx-3 flex flex-wrap items-center gap-2 rounded-xl border border-line px-3 py-2 text-sm">
      <span className="text-dim">
        {t('assistant.proposedNote')}
        {proposal.title != null && proposal.title !== '' && (
          <>
            {' · '}
            <b className="font-semibold text-text">{proposal.title}</b>
          </>
        )}
      </span>
      <span className="ml-auto">
        {applied !== null ? (
          <AppliedMark applied={applied} label={t('assistant.noteKept')} />
        ) : (
          <Button
            size="sm"
            variant="primary"
            disabled={keep.isPending}
            onClick={() => {
              keep.mutate(undefined)
            }}
          >
            {t('assistant.addNote')}
          </Button>
        )}
      </span>
    </div>
  )
}
