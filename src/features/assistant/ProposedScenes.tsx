import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import type { Applied, ScenesProposal } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { useApplyProposal } from '@/lib/useApplyProposal'
import { Button } from '@/components/ui/button'
import { ProposalCard } from '@/features/assistant/ProposalCard'

interface Props {
  /** The work the chat is on: a storyboard is always for one. */
  workId: string
  messageId: string
  proposal: ScenesProposal
  /** Set once somebody applied it; read from the message. */
  applied: Applied | null
  /** Set once somebody turned it down; read from the message. */
  dismissed: boolean
}

/**
 * A storyboard an agent proposed, next to the one button that writes it.
 *
 * The message above holds the board as a table and every prompt block, so
 * it is read before a row is written; this is the count and the decision.
 * The decision differs by what the agent asked: *add to the board* keeps
 * what is there and numbers the new scenes after it, *replace the board*
 * rewrites a scene with the same number in place and sends the rest to the
 * trash — said above the button, because a replaced board is not undone
 * with one Ctrl+Z. Once applied, the board is one click away.
 */
export function ProposedScenes({ workId, messageId, proposal, applied, dismissed }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  // A proposal stored by v0.62 said `replace: true` where v0.64 says
  // `change: 'replace'`; read as it was meant. The backend's own type no
  // longer carries the old field at all - this is reading a message stored
  // before the rename, not the current shape - so the cast is by hand.
  const legacyReplace = (proposal as { replace?: boolean }).replace
  const change = proposal.change ?? (legacyReplace === true ? 'replace' : 'add')
  const words = {
    add: {
      title: 'assistant.proposedScenes',
      button: 'assistant.addScenes',
      done: 'assistant.scenesAdded',
      mark: 'assistant.scenesAddedMark',
    },
    replace: {
      title: 'assistant.proposedScenesReplace',
      button: 'assistant.replaceScenes',
      done: 'assistant.scenesReplaced',
      mark: 'assistant.scenesReplacedMark',
    },
    revise: {
      title: 'assistant.proposedScenesRevise',
      button: 'assistant.reviseScenes',
      done: 'assistant.scenesRevised',
      mark: 'assistant.scenesRevisedMark',
    },
  }[change]

  const apply = useApplyProposal({
    messageId,
    message: t(words.done),
    refresh: [keys.scenes, keys.work(workId), keys.deletions],
  })

  return (
    <ProposalCard
      messageId={messageId}
      title={t(words.title)}
      warnings={change === 'replace' ? [t('assistant.scenesReplaceHint')] : []}
      applied={applied}
      dismissed={dismissed}
      applyLabel={t(words.button)}
      onApply={() => {
        apply.mutate(undefined)
      }}
      applying={apply.isPending}
      appliedLabel={t(words.mark)}
      afterApplied={
        <Button
          size="xs"
          variant="ghost"
          onClick={() => {
            void navigate(`/works/${workId}/scenes`)
          }}
        >
          {t('assistant.openBoard')}
        </Button>
      }
    >
      <p>{t('assistant.packageScenes', { count: proposal.scenes.length })}</p>
    </ProposalCard>
  )
}
