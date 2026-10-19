import { useTranslation } from 'react-i18next'
import type { Applied, NoteProposal } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { useApplyProposal } from '@/lib/useApplyProposal'
import { labelOf, useProfile } from '@/lib/useProfile'
import { ProposalCard } from '@/features/assistant/ProposalCard'

interface Props {
  messageId: string
  proposal: NoteProposal
  /** Set once somebody applied it; read from the message. */
  applied: Applied | null
  /** Set once somebody turned it down; read from the message. */
  dismissed: boolean
}

/**
 * A note an agent proposed, next to the button that keeps it.
 *
 * Same bargain as a proposed score: the text is readable before it is
 * written, and a person writes it. Kept, it is an ordinary note — the same
 * table, no mark saying a machine suggested it — on the chat's work, or on
 * nothing when the chat is about nothing. The toast offers to take it back,
 * since adding a note is one of the things undo covers.
 *
 * A phrase says it is one, whose words it is and the shelf it goes on (ADR
 * 0064): what keeping it writes is read before it is written.
 */
export function ProposedNote({ messageId, proposal, applied, dismissed }: Props) {
  const { t } = useTranslation()
  const { config } = useProfile()
  const kinds = config.note_kinds ?? []
  const said = [
    proposal.note_kind != null ? labelOf(kinds, proposal.note_kind) : null,
    proposal.attribution != null ? `— ${proposal.attribution}` : null,
    proposal.shelf != null ? t('assistant.noteOnShelf', { shelf: proposal.shelf }) : null,
  ].filter((part) => part !== null)

  const keep = useApplyProposal({
    messageId,
    message: t('assistant.noteAdded'),
    refresh: [keys.notes],
  })

  return (
    <ProposalCard
      messageId={messageId}
      title={t('assistant.proposedNote')}
      applied={applied}
      dismissed={dismissed}
      applyLabel={t('assistant.addNote')}
      onApply={() => {
        keep.mutate(undefined)
      }}
      applying={keep.isPending}
      appliedLabel={t('assistant.noteKept')}
    >
      {proposal.title != null && proposal.title !== '' && (
        <b className="font-semibold text-text">{proposal.title}</b>
      )}
      {said.length > 0 && <p className="text-xs text-dim">{said.join(' · ')}</p>}
    </ProposalCard>
  )
}
