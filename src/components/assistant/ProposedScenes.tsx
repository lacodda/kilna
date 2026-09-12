import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import type { Applied, ScenesProposal } from '@/lib/api'
import { keys } from '@/lib/query'
import { useApplyProposal } from '@/lib/useApplyProposal'
import { Button } from '@/components/ui/button'
import { AppliedMark } from '@/components/assistant/AppliedMark'

interface Props {
  /** The work the chat is on: a storyboard is always for one. */
  workId: string
  messageId: string
  proposal: ScenesProposal
  /** Set once somebody applied it; read from the message. */
  applied: Applied | null
}

/**
 * A storyboard an agent proposed, next to the one button that writes it.
 *
 * The message above holds the board as a table and every prompt block, so
 * it is read before a row is written; this is the count and the decision.
 * The decision differs by what the agent asked: *add to the board* keeps
 * what is there and numbers the new scenes after it, *replace the board*
 * rewrites a scene with the same number in place and sends the rest to the
 * trash — said under the button, because a replaced board is not undone
 * with one Ctrl+Z. Once applied, the board is one click away.
 */
export function ProposedScenes({ workId, messageId, proposal, applied }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const replace = proposal.replace === true

  const apply = useApplyProposal({
    messageId,
    message: t(replace ? 'assistant.scenesReplaced' : 'assistant.scenesAdded'),
    refresh: [keys.scenes, keys.work(workId), keys.deletions],
  })

  return (
    <div className="mx-3 flex flex-col gap-1.5 rounded-xl border border-line bg-soft px-3 py-2 text-sm">
      <p className="text-xs font-semibold text-dim">
        {t(replace ? 'assistant.proposedScenesReplace' : 'assistant.proposedScenes')}
        {' · '}
        <span className="font-normal">
          {t('assistant.packageScenes', { count: proposal.scenes.length })}
        </span>
      </p>
      {replace && applied === null && (
        <p className="text-xs text-warn">{t('assistant.scenesReplaceHint')}</p>
      )}
      <div className="flex items-center justify-end gap-2">
        {applied !== null ? (
          <>
            <AppliedMark
              applied={applied}
              label={t(replace ? 'assistant.scenesReplacedMark' : 'assistant.scenesAddedMark')}
            />
            <Button
              size="sm"
              variant="icon"
              className="h-6 px-1.5 text-[11px]"
              onClick={() => {
                void navigate(`/works/${workId}/scenes`)
              }}
            >
              {t('assistant.openBoard')}
            </Button>
          </>
        ) : (
          <Button
            size="sm"
            variant="primary"
            disabled={apply.isPending}
            onClick={() => {
              apply.mutate(undefined)
            }}
          >
            {t(replace ? 'assistant.replaceScenes' : 'assistant.addScenes')}
          </Button>
        )}
      </div>
    </div>
  )
}
