import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { X } from 'lucide-react'
import { deleteScore, listVersions, scoreHistory, scoreWork } from '@/lib/api'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { announceDeleted } from '@/lib/trash'
import { labelOf, useProfile } from '@/lib/useProfile'
import { tierFor, total as computeTotal } from '@/lib/scoring'
import { Badge } from '@/components/ui/Badge'
import { Button } from '@/components/ui/Button'
import { Input } from '@/components/ui/Input'
import { Panel } from '@/components/ui/Panel'
import { SegmentedScale } from '@/components/ui/SegmentedScale'
import { Select } from '@/components/ui/Select'
import { Skeleton } from '@/components/ui/Skeleton'
import { Sparkline } from '@/components/ui/Sparkline'
import { cn } from '@/lib/utils'

interface Props {
  workId: string
}

/**
 * Judging a work along the profile's axes.
 *
 * One row per axis — what it is called, what it weighs, what it asks — with a
 * scale you click rather than a box you type into. Scoring is a verdict, and
 * the interface should read like one being given.
 */
export function ScorePanel({ workId }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const client = useQueryClient()
  const { axes, tiers } = profile.config

  const [values, setValues] = useState<Record<string, number>>({})
  const [note, setNote] = useState('')
  // Empty means "whatever the work currently points at", which is what the
  // backend already does when no version is named.
  const [versionId, setVersionId] = useState('')

  const history = useQuery({
    queryKey: keys.scoreHistory(workId),
    queryFn: () => scoreHistory(workId),
  })

  const versions = useQuery({
    queryKey: keys.versions(workId),
    queryFn: () => listVersions(workId),
  })

  const filled = Object.keys(values).length
  const preview = computeTotal(axes, values)
  const previewTier = tierFor(tiers, preview)

  // A score moves the catalogue and the work's own summary, either way — and
  // leaves a line in the journal, which the card shows underneath.
  const refreshed = [
    keys.journal,
    keys.scoreHistory(workId),
    keys.latestScore(workId),
    keys.catalogue,
    keys.works,
  ]

  const settle = () => {
    for (const key of refreshed) void client.invalidateQueries({ queryKey: key })
  }

  const save = useMutation({
    mutationFn: () =>
      scoreWork(workId, {
        axes: values,
        version_id: versionId === '' ? null : versionId,
        note: note.trim() === '' ? null : note.trim(),
      }),
    onSuccess: () => {
      setValues({})
      setNote('')
      setVersionId('')
      settle()
      say.ok(t('toast.scoreSaved'))
    },
    onError: (cause) => say.failedTo(t('toast.scoreSaveFailed'), cause),
  })

  const remove = useMutation({
    mutationFn: deleteScore,
    onSuccess: (deletionId) =>
      announceDeleted({
        client,
        deletionId,
        message: t('toast.scoreDeleted'),
        refresh: refreshed,
      }),
    onError: (cause) => say.failedTo(t('toast.scoreSaveFailed'), cause),
  })

  const historyData = history.data ?? []
  // The previous total, to show what a revision did to it.
  const previous = historyData[1]?.total
  // Oldest first for the line; the list below stays newest first.
  const trend = [...historyData].reverse().map((score) => score.total)

  const versionOptions = (versions.data ?? []).map((version) => ({
    value: version.id,
    label: `${labelOf(profile.config.version_roles, version.role)} · ${
      version.label ?? t('versions.revision', { number: version.revision })
    }${version.is_current ? ` · ${t('versions.current')}` : ''}`,
  }))

  return (
    <section className="flex flex-col gap-3">
      <h3 className="text-sm font-semibold">{t('score.title')}</h3>

      {history.isError && (
        <p role="alert" className="text-sm text-bad">
          {t('toast.loadFailed')}
        </p>
      )}

      <Panel className="overflow-hidden">
        <div className="flex flex-col gap-2.5 p-4">
          {axes.map((axis) => (
            <div
              key={axis.key}
              className="grid items-center gap-3 sm:grid-cols-[minmax(10rem,16rem)_minmax(0,1fr)_2.5rem]"
            >
              <span className="min-w-0">
                <b className="block truncate text-[13px] font-semibold">
                  {axis.label}{' '}
                  <span className="font-mono text-[10.5px] font-normal text-faint">
                    ×{axis.weight}
                  </span>
                </b>
                {/* The question the axis asks, in the open. It used to be a
                    tooltip, which is the same as not being there — but wrapping
                    it made a six-axis card taller than the screen, and scoring
                    is a judgement you make by looking at all the axes at once.
                    One line, with the whole of it on hover. */}
                {axis.description !== undefined && axis.description !== '' && (
                  <span className="block truncate text-[11px] text-faint" title={axis.description}>
                    {axis.description}
                  </span>
                )}
              </span>

              <SegmentedScale
                scale={axis.scale}
                value={values[axis.key]}
                label={axis.label}
                onChange={(next) =>
                  setValues((current) => {
                    const updated = { ...current }
                    if (next === undefined) delete updated[axis.key]
                    else updated[axis.key] = next
                    return updated
                  })
                }
              />

              <span className="text-right font-mono text-[13px] text-dim tabular-nums">
                {values[axis.key] ?? '—'}
              </span>
            </div>
          ))}
        </div>

        <div className="flex flex-col gap-3 border-t border-line p-4">
          <div className="flex flex-wrap items-center gap-3">
            <span className="font-mono text-[22px] font-semibold tabular-nums">
              {filled === 0 ? '—' : preview.toFixed(1)}
            </span>

            {previewTier !== undefined && filled > 0 && (
              <Badge variant="accent">{previewTier.label}</Badge>
            )}

            {trend.length > 1 && <Sparkline values={trend} />}

            <Button
              className="ml-auto"
              variant="primary"
              disabled={filled === 0 || save.isPending}
              onClick={() => save.mutate()}
            >
              {t('score.save')}
            </Button>
          </div>

          {filled > 0 && filled < axes.length && (
            <p className="text-xs text-dim">{t('score.partial', { filled, count: axes.length })}</p>
          )}

          {/* Both of these have been in the API since v0.3.0 and never sent.
              A score belongs to the draft it judged — usually the current one,
              which is what an empty choice means. */}
          <div className="flex flex-wrap items-end gap-3">
            <label className="flex flex-col gap-1">
              <span className="text-[10px] font-semibold uppercase tracking-[0.08em] text-faint">
                {t('score.ofVersion')}
              </span>
              <Select
                className="w-64"
                aria-label={t('score.ofVersion')}
                value={versionId}
                onChange={setVersionId}
                placeholder={t('score.currentVersion')}
                options={versionOptions}
              />
            </label>

            <label className="flex min-w-56 flex-1 flex-col gap-1">
              <span className="text-[10px] font-semibold uppercase tracking-[0.08em] text-faint">
                {t('score.note')}
              </span>
              <Input
                value={note}
                onChange={(event) => setNote(event.target.value)}
                placeholder={t('score.notePlaceholder')}
              />
            </label>
          </div>
        </div>
      </Panel>

      {history.isPending && <Skeleton className="h-24 w-full" />}

      {historyData.length > 0 && (
        <ul className="flex flex-col gap-1">
          {historyData.map((score, index) => {
            const delta = index === 0 && previous !== undefined ? score.total - previous : undefined

            return (
              <li
                key={score.id}
                className="flex flex-wrap items-center gap-3 rounded-xl border border-line px-3 py-1.5 text-sm"
              >
                <span className="w-14 font-semibold tabular-nums">{score.total.toFixed(1)}</span>
                {score.tier !== null && (
                  <span className="rounded bg-soft px-1.5 py-0.5 text-xs">
                    {labelOf(tiers, score.tier)}
                  </span>
                )}
                {score.revision !== null && score.version_id !== null && (
                  // The judgement points at what was judged. A score is about a
                  // particular draft, and the review of that draft is the rest
                  // of the sentence this number starts.
                  <Link
                    to={`/works/${workId}/versions?version=${score.version_id}`}
                    title={t('score.openVersion')}
                    className="text-xs text-dim underline decoration-dotted underline-offset-2 transition-colors hover:text-text"
                  >
                    {t('versions.revision', { number: score.revision })}
                  </Link>
                )}
                {score.revision !== null && score.version_id === null && (
                  <span className="text-xs text-dim">
                    {t('versions.revision', { number: score.revision })}
                  </span>
                )}
                {delta !== undefined && Math.abs(delta) >= 0.05 && (
                  <span className={cn('text-xs font-medium', delta > 0 ? 'text-good' : 'text-bad')}>
                    {delta > 0 ? '+' : ''}
                    {delta.toFixed(1)}
                  </span>
                )}
                {score.note !== null && score.note !== '' && (
                  <span className="min-w-0 basis-full text-xs text-dim sm:basis-auto">
                    “{score.note}”
                  </span>
                )}
                <span className="ml-auto text-xs text-dim">{score.scored_at.slice(0, 10)}</span>
                <Button
                  variant="danger"
                  size="iconSm"
                  title={t('score.delete')}
                  aria-label={t('score.delete')}
                  onClick={() => remove.mutate(score.id)}
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
