import { useTranslation } from 'react-i18next'
import type { Applied, ScoreProposal } from '@/lib/api/types'
import { formatTotal } from '@/lib/format'
import { keys } from '@/lib/query/keys'
import { total } from '@/lib/scoring'
import { useApplyProposal } from '@/lib/useApplyProposal'
import { say, useVocabulary } from '@/lib/useProfile'
import { ProposalCard } from '@/features/assistant/ProposalCard'

interface Props {
  workId: string
  messageId: string
  proposal: ScoreProposal
  /** Set once somebody applied it; read from the message. */
  applied: Applied | null
  /** Set once somebody turned it down; read from the message. */
  dismissed: boolean
}

/**
 * A score the assistant proposed, next to the button that applies it.
 *
 * The assistant never writes to the workspace — it says what it would score,
 * and a person decides. That is why this is a card with numbers on it rather
 * than a line saying "scored": what is about to be written is readable before
 * it is written, exactly as a rendered prompt is readable before it is sent.
 * The total at the top is the one the score screen would show, from the same
 * weights; the backend recomputes it on the way in.
 *
 * Applied, it becomes an ordinary snapshot: same table, same history. The
 * note carries the "why" in the assistant's own words, because "why" is the
 * part a bare number cannot hold; a score from an agent outside the window
 * names that agent as its judge, so the history does not read it as the
 * author's own.
 */
export function ProposedScore({ workId, messageId, proposal, applied, dismissed }: Props) {
  const { t } = useTranslation()
  const axes = useVocabulary(workId).axes

  const apply = useApplyProposal({
    messageId,
    message: t('assistant.scoreApplied'),
    // One coarse prefix over every score query: the history, the latest, and
    // whatever the catalogue derived from them all moved at once.
    refresh: [keys.scores, keys.work(workId), keys.works, keys.catalogue],
  })

  // The profile's own label for each axis, so the card reads the way the
  // score screen does rather than showing raw keys.
  const named = Object.entries(proposal.axes).map(([key, value]) => ({
    key,
    label: axes.find((axis) => axis.key === key)?.label ?? key,
    scale: axes.find((axis) => axis.key === key)?.scale,
    value,
  }))

  const warnings = [
    ...(proposal.missing !== undefined && proposal.missing.length > 0
      ? [t('assistant.scoreMissing', { axes: proposal.missing.join(', ') })]
      : []),
    ...(proposal.unknown !== undefined && proposal.unknown.length > 0
      ? [t('assistant.scoreUnknown', { axes: proposal.unknown.join(', ') })]
      : []),
  ]

  return (
    <ProposalCard
      messageId={messageId}
      title={t('assistant.scoreTitle')}
      // Only when the profile knows an axis it names: a total over nothing
      // would be a confident zero.
      figure={
        named.some((axis) => axis.scale !== undefined)
          ? formatTotal(total(axes, proposal.axes))
          : undefined
      }
      warnings={warnings}
      applied={applied}
      dismissed={dismissed}
      applyLabel={t('assistant.scoreApply')}
      onApply={() => {
        apply.mutate(undefined)
      }}
      applying={apply.isPending}
      appliedLabel={t('assistant.scoreApplied')}
    >
      <dl className="flex flex-wrap gap-x-3 gap-y-0.5">
        {named.map((axis) => (
          <div key={axis.key} className="flex items-baseline gap-1">
            <dt>{say(axis.label)}</dt>
            <dd className="font-mono text-text tabular-nums">
              {axis.value}
              {axis.scale !== undefined && <span className="text-faint">/{axis.scale}</span>}
            </dd>
          </div>
        ))}
      </dl>
      {proposal.note !== undefined && <p>{proposal.note}</p>}
    </ProposalCard>
  )
}
