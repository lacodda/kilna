import { useTranslation } from 'react-i18next'
import type { Applied, ScoreProposal } from '@/lib/api'
import { keys } from '@/lib/query'
import { useApplyProposal } from '@/lib/useApplyProposal'
import { say, useVocabulary } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { AppliedMark } from '@/components/assistant/AppliedMark'

interface Props {
  workId: string
  messageId: string
  proposal: ScoreProposal
  /** Set once somebody applied it; read from the message. */
  applied: Applied | null
}

/**
 * A score the assistant proposed, next to the button that applies it.
 *
 * The assistant never writes to the workspace — it says what it would score,
 * and a person decides. That is why this is a panel with numbers on it rather
 * than a line saying "scored": what is about to be written is readable before
 * it is written, exactly as a rendered prompt is readable before it is sent.
 *
 * Applied, it becomes an ordinary snapshot: same table, same history. The
 * note carries the "why" in the assistant's own words, because "why" is the
 * part a bare number cannot hold; a score from an agent outside the window
 * names that agent as its judge, so the history does not read it as the
 * author's own.
 */
export function ProposedScore({ workId, messageId, proposal, applied }: Props) {
  const { t } = useTranslation()
  const axes = useVocabulary(workId).axes

  const apply = useApplyProposal({
    messageId,
    message: t('assistant.scoreApplied'),
    // One coarse prefix over every score query: the history, the latest, and
    // whatever the catalogue derived from them all moved at once.
    refresh: [keys.scores, keys.work(workId), keys.works, keys.catalogue],
  })

  // The profile's own label for each axis, so the panel reads the way the
  // score screen does rather than showing raw keys.
  const named = Object.entries(proposal.axes).map(([key, value]) => ({
    key,
    label: axes.find((axis) => axis.key === key)?.label ?? key,
    scale: axes.find((axis) => axis.key === key)?.scale,
    value,
  }))

  return (
    <div className="rounded-xl border border-line bg-soft px-3 py-2">
      <p className="text-xs font-semibold text-dim">{t('assistant.scoreTitle')}</p>

      <dl className="mt-1.5 flex flex-wrap gap-x-4 gap-y-1">
        {named.map((axis) => (
          <div key={axis.key} className="flex items-baseline gap-1.5">
            <dt className="text-xs text-dim">{say(axis.label)}</dt>
            <dd className="text-sm tabular-nums">
              {axis.value}
              {axis.scale !== undefined && (
                <span className="text-xs text-faint">/{axis.scale}</span>
              )}
            </dd>
          </div>
        ))}
      </dl>

      {proposal.note !== undefined && (
        <p className="mt-1.5 text-xs text-dim">{proposal.note}</p>
      )}

      {/* A proposal that only half fits is still worth applying — but never
          without saying so. */}
      {proposal.missing !== undefined && proposal.missing.length > 0 && (
        <p className="mt-1.5 text-xs text-warn">
          {t('assistant.scoreMissing', { axes: proposal.missing.join(', ') })}
        </p>
      )}
      {proposal.unknown !== undefined && proposal.unknown.length > 0 && (
        <p className="mt-1 text-xs text-warn">
          {t('assistant.scoreUnknown', { axes: proposal.unknown.join(', ') })}
        </p>
      )}

      <div className="mt-2 flex justify-end">
        {applied !== null ? (
          <AppliedMark applied={applied} label={t('assistant.scoreApplied')} />
        ) : (
          <Button
            size="sm"
            variant="primary"
            disabled={apply.isPending}
            onClick={() => {
              apply.mutate(undefined)
            }}
          >
            {t('assistant.scoreApply')}
          </Button>
        )}
      </div>
    </div>
  )
}
