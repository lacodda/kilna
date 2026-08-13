import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { X } from 'lucide-react'
import { deleteScore, scoreHistory, scoreWork, type Score } from '@/lib/api'
import { labelOf, useProfile } from '@/lib/useProfile'
import { tierFor, total as computeTotal } from '@/lib/scoring'
import { Button } from '@/components/ui/Button'
import { Input } from '@/components/ui/Input'
import { cn } from '@/lib/utils'

interface Props {
  workId: string
  onChanged: () => void
}

export function ScorePanel({ workId, onChanged }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const { axes, tiers } = profile.config

  const [values, setValues] = useState<Record<string, number>>({})
  const [history, setHistory] = useState<Score[]>([])
  const [error, setError] = useState<string | null>(null)
  const [revision, setRevision] = useState(0)

  useEffect(() => {
    scoreHistory(workId)
      .then((result) => {
        setHistory(result)
        setError(null)
      })
      .catch((cause: unknown) => setError(String(cause)))
  }, [workId, revision])

  const filled = Object.keys(values).length
  const preview = computeTotal(axes, values)
  const previewTier = tierFor(tiers, preview)

  const save = () => {
    scoreWork(workId, { axes: values })
      .then(() => {
        setValues({})
        setRevision((current) => current + 1)
        onChanged()
      })
      .catch((cause: unknown) => setError(String(cause)))
  }

  // The previous total, to show what a revision did to it.
  const previous = history[1]?.total

  return (
    <section className="flex flex-col gap-3">
      <h3 className="text-sm font-semibold">{t('score.title')}</h3>

      {error !== null && (
        <p role="alert" className="text-sm text-bad">
          {error}
        </p>
      )}

      <div className="flex flex-wrap items-end gap-3">
        {axes.map((axis) => (
          <label key={axis.key} className="flex flex-col gap-1" title={axis.description}>
            <span className="text-xs font-medium uppercase tracking-wide text-dim">
              {axis.label} <span className="normal-case text-faint">×{axis.weight}</span>
            </span>
            <Input
              className="w-20"
              type="number"
              min={0}
              max={axis.scale}
              step={0.5}
              value={values[axis.key] ?? ''}
              onChange={(event) => {
                const raw = event.target.value
                setValues((current) => {
                  const next = { ...current }
                  if (raw === '') {
                    delete next[axis.key]
                  } else {
                    // Clamped, so a stray keystroke cannot invent a 90 out of 10.
                    next[axis.key] = Math.min(Math.max(Number(raw), 0), axis.scale)
                  }
                  return next
                })
              }}
            />
          </label>
        ))}

        <div className="ml-auto flex items-end gap-3">
          <div className="text-right">
            <span className="block text-xs uppercase tracking-wide text-dim">
              {t('score.total')}
            </span>
            <span className="text-xl font-semibold tabular-nums">
              {filled === 0 ? '—' : preview.toFixed(1)}
            </span>
            {previewTier !== undefined && filled > 0 && (
              <span className="ml-2 rounded bg-accent-soft px-1.5 py-0.5 text-xs">
                {previewTier.label}
              </span>
            )}
          </div>
          <Button variant="primary" disabled={filled === 0} onClick={save}>
            {t('score.save')}
          </Button>
        </div>
      </div>

      {filled > 0 && filled < axes.length && (
        <p className="text-xs text-dim">
          {t('score.partial', { filled, count: axes.length })}
        </p>
      )}

      {history.length > 0 && (
        <ul className="flex flex-col gap-1">
          {history.map((score, index) => {
            const delta =
              index === 0 && previous !== undefined ? score.total - previous : undefined

            return (
              <li
                key={score.id}
                className="flex items-center gap-3 rounded-xl border border-line px-3 py-1.5 text-sm"
              >
                <span className="w-14 font-semibold tabular-nums">{score.total.toFixed(1)}</span>
                {score.tier !== null && (
                  <span className="rounded bg-soft px-1.5 py-0.5 text-xs">
                    {labelOf(tiers, score.tier)}
                  </span>
                )}
                {score.revision !== null && (
                  <span className="text-xs text-dim">
                    {t('versions.revision', { number: score.revision })}
                  </span>
                )}
                {delta !== undefined && Math.abs(delta) >= 0.05 && (
                  <span
                    className={cn('text-xs font-medium', delta > 0 ? 'text-good' : 'text-bad')}
                  >
                    {delta > 0 ? '+' : ''}
                    {delta.toFixed(1)}
                  </span>
                )}
                <span className="ml-auto text-xs text-dim">
                  {score.scored_at.slice(0, 10)}
                </span>
                <Button
                  variant="danger"
                  size="iconSm"
                  title={t('score.delete')}
                  onClick={() => {
                    deleteScore(score.id)
                      .then(() => {
                        setRevision((current) => current + 1)
                        onChanged()
                      })
                      .catch((cause: unknown) => setError(String(cause)))
                  }}
                >
                  <X aria-hidden className="size-3.5" />
                </Button>
              </li>
            )
          })}
        </ul>
      )}
    </section>
  )
}
