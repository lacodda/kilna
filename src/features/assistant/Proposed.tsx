import type { Exchange } from '@/lib/chat'
import { ProposedCanon, ProposedCardPrompt } from '@/features/assistant/ProposedCanon'
import { ProposedCommentOrReply } from '@/features/assistant/ProposedCommentOrReply'
import { ProposedCoverIdeas } from '@/features/assistant/ProposedCoverIdeas'
import { ProposedTrials } from '@/features/assistant/ProposedTrials'
import { ProposedDescription } from '@/features/assistant/ProposedDescription'
import { ProposedNote } from '@/features/assistant/ProposedNote'
import { ProposedRelease } from '@/features/assistant/ProposedRelease'
import { ProposedScenes } from '@/features/assistant/ProposedScenes'
import { ProposedScore } from '@/features/assistant/ProposedScore'
import { ProposedVersion } from '@/features/assistant/ProposedVersion'
import { ProposedWork } from '@/features/assistant/ProposedWork'
import { ProposedWords } from '@/features/assistant/ProposedWords'
import { ProposedBricks } from '@/features/assistant/ProposedBricks'

interface Props {
  /** The settled answer that carries the proposal. */
  answer: NonNullable<Exchange['answer']>
  /** The work the chat is on. Absent when it is about nothing: a score, a
   * version or a storyboard then has nothing to land on, and is not offered. */
  workId?: string
  /** Opens the dialog that inserts the answer as a version, for a proposed
   * version whose role or name the person wants to choose first. */
  onChooseVersion: (role: string, label: string | undefined) => void
}

/**
 * The card for whatever an answer proposed - one per kind, every one of them
 * a `ProposalCard`, so each reads and answers the same way.
 */
export function Proposed({ answer, workId, onChooseVersion }: Props) {
  const { id: messageId, proposal, applied } = answer
  const dismissed = answer.dismissed !== null
  if (proposal === null) return null

  switch (proposal.kind) {
    case 'score':
      return workId === undefined ? null : (
        <ProposedScore
          workId={workId}
          messageId={messageId}
          proposal={proposal}
          applied={applied}
          dismissed={dismissed}
        />
      )
    case 'version':
      return workId === undefined ? null : (
        <ProposedVersion
          workId={workId}
          messageId={messageId}
          proposal={proposal}
          applied={applied}
          dismissed={dismissed}
          onChoose={() => {
            onChooseVersion(proposal.role, proposal.label ?? undefined)
          }}
        />
      )
    case 'scenes':
      return workId === undefined ? null : (
        <ProposedScenes
          workId={workId}
          messageId={messageId}
          proposal={proposal}
          applied={applied}
          dismissed={dismissed}
        />
      )
    case 'note':
      return (
        <ProposedNote
          messageId={messageId}
          proposal={proposal}
          applied={applied}
          dismissed={dismissed}
        />
      )
    case 'work':
      return (
        <ProposedWork
          workId={workId}
          messageId={messageId}
          proposal={proposal}
          applied={applied}
          dismissed={dismissed}
        />
      )
    case 'comment':
    case 'reply':
      return (
        <ProposedCommentOrReply
          messageId={messageId}
          proposal={proposal}
          applied={applied}
          dismissed={dismissed}
        />
      )
    case 'description':
      return <ProposedDescription messageId={messageId} applied={applied} dismissed={dismissed} />
    case 'canon':
      return (
        <ProposedCanon
          messageId={messageId}
          proposal={proposal}
          applied={applied}
          dismissed={dismissed}
        />
      )
    case 'cardPrompt':
      return (
        <ProposedCardPrompt
          messageId={messageId}
          noteId={proposal.note_id}
          applied={applied}
          dismissed={dismissed}
        />
      )
    case 'coverIdeas':
      return (
        <ProposedCoverIdeas
          messageId={messageId}
          proposal={proposal}
          applied={applied}
          dismissed={dismissed}
        />
      )
    case 'trials':
      return (
        <ProposedTrials
          messageId={messageId}
          proposal={proposal}
          applied={applied}
          dismissed={dismissed}
        />
      )
    case 'words':
      return (
        <ProposedWords
          messageId={messageId}
          proposal={proposal}
          applied={applied}
          dismissed={dismissed}
        />
      )
    case 'bricks':
      return (
        <ProposedBricks
          messageId={messageId}
          proposal={proposal}
          applied={applied}
          dismissed={dismissed}
        />
      )
    case 'release':
      return (
        <ProposedRelease
          messageId={messageId}
          proposal={proposal}
          applied={applied}
          dismissed={dismissed}
          workId={workId}
        />
      )
  }
}
