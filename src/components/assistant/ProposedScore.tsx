import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Check } from 'lucide-react'
import { scoreWork, type ScoreProposal } from '@/lib/api'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/Button'

interface Props {
  workId: string
  proposal: ScoreProposal
}

/**
 * A score the assistant proposed, next to the button that applies it.
 *
 * The assistant never writes to the workspace — it says what it would score,
 * and a person decides. That is why this is a panel with numbers on it rather
 * than a line saying "scored": what is about to be written is readable before
 * it is written, exactly as a rendered prompt is readable before it is sent.
 *
 * Applied, it becomes an ordinary snapshot: same table, same history, no mark
 * saying a machine suggested it. The note carries that, in the assistant's own
 * words, because "why" is the part a bare number cannot hold.
 */
export function ProposedScore({ workId, proposal }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const client = useQueryClient()
  const [applied, setApplied] = useState(false)

  const apply = useMutation({
    mutationFn: () =>
      scoreWork(workId, {
        axes: proposal.axes,
        note: proposal.note ?? null,
      }),
    onSuccess: () => {
      setApplied(true)
      // One coarse prefix over every score query: the history, the latest, and
      // whatever the catalogue derived from them all moved at once.
      void client.invalidateQueries({ queryKey: keys.scores })
      void client.invalidateQueries({ queryKey: keys.work(workId) })
      void client.invalidateQueries({ queryKey: keys.works })
      void client.invalidateQueries({ queryKey: keys.catalogue })
      say.ok(t('assistant.scoreApplied'))
    },
    onError: (cause) => {
      say.failed(cause)
    },
  })

  // The profile's own label for each axis, so the panel reads the way the
  // score screen does rather than showing raw keys.
  const named = Object.entries(proposal.axes).map(([key, value]) => ({
    key,
    label: profile.config.axes.find((axis) => axis.key === key)?.label ?? key,
    scale: profile.config.axes.find((axis) => axis.key === key)?.scale,
    value,
  }))

  return (
    <div className="rounded-xl border border-line bg-soft px-3 py-2">
      <p className="text-xs font-semibold text-dim">{t('assistant.scoreTitle')}</p>

      <dl className="mt-1.5 flex flex-wrap gap-x-4 gap-y-1">
        {named.map((axis) => (
          <div key={axis.key} className="flex items-baseline gap-1.5">
            <dt className="text-xs text-dim">{axis.label}</dt>
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
        {applied ? (
          <span className="flex items-center gap-1 text-xs text-dim">
            <Check aria-hidden className="size-3.5" />
            {t('assistant.scoreApplied')}
          </span>
        ) : (
          <Button
            size="sm"
            variant="primary"
            disabled={apply.isPending}
            onClick={() => {
              apply.mutate()
            }}
          >
            {t('assistant.scoreApply')}
          </Button>
        )}
      </div>
    </div>
  )
}
