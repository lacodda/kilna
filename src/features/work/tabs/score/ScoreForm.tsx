import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { Score, Work } from '@/lib/api/types'
import { scoreWork } from '@/lib/api/scores'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import {
  defaultJudged,
  judgedVersions,
  markReaching,
  tierFor,
  toNextTier,
  total as computeTotal,
} from '@/lib/scoring'
import { say } from '@/lib/toast'
import { labelOf, say as sayLabel, useVocabulary } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Field, FieldGroup } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Select } from '@/components/AppSelect'
import { Pane } from '@/components/frame'
import { AxisGrid, AxisRow } from '@/features/work/tabs/score/AxisRow'
import { ScoreFoot } from '@/features/work/tabs/score/ScoreFoot'

interface Props {
  workId: string
  trail: Score[]
  /** An axis over the recorded scores, oldest first; nothing while blind. */
  trend: (axis: string) => number[] | undefined
  work: Work | undefined
  /** The tier the standing score arrives at, for the pin. */
  scored: string | null
  /** Who is judging, kept by the tab across scores. */
  rater: string
  onRater: (rater: string) => void
  /** The score is recorded: the tab opens it. */
  onSaved: () => void
}

/**
 * A new score being given: empty scales, and the verdict they add up to
 * standing at the foot as they are marked.
 *
 * Empty, not a copy of the last score. Starting from the recorded marks made
 * every new score an edit of the old one - an anchor the eye cannot unsee,
 * which is the bias judging blind exists to take away - and it made "am I
 * reading or judging" a question the panel had to answer in small print.
 */
export function ScoreForm({ workId, trail, trend, work, scored, rater, onRater, onSaved }: Props) {
  const { t } = useTranslation()
  const { axes, tiers, version_roles: roles } = useVocabulary(workId)

  const [values, setValues] = useState<Record<string, number>>({})
  const [note, setNote] = useState('')
  // Empty means "the default below", not "no version": the choice is only
  // stored once the person makes one.
  const [chosen, setChosen] = useState('')

  const versions = useQuery(queries.versions(workId))
  // Only a draft of the work can be judged - not its review, its critique or
  // its style prompt - and the profile says which roles those are.
  const judged = judgedVersions(versions.data ?? [], roles)
  const versionId = chosen !== '' ? chosen : (defaultJudged(judged)?.id ?? '')

  const filled = Object.keys(values).length
  const preview = computeTotal(axes, values)
  // What the next tier costs, and where it is cheapest - the advice this
  // tab exists to give once the number stops being interesting on its own.
  const ahead = filled === 0 ? undefined : toNextTier(axes, values, tiers, preview)

  const save = useAppMutation({
    mutationFn: () =>
      scoreWork(workId, {
        axes: values,
        // Empty only when the work has no draft to judge yet, and then the
        // backend attaches the work's current version, as it always has.
        version_id: versionId === '' ? undefined : versionId,
        note: note.trim() === '' ? undefined : note.trim(),
        rater: rater.trim() === '' ? undefined : rater.trim(),
      }),
    failure: 'toast.scoreSaveFailed',
    refresh: refresh.score(workId),
    onSuccess: () => {
      say.ok(t('toast.scoreSaved'))
      onSaved()
    },
  })

  const mark = (key: string) => (next: number | undefined) =>
    setValues((current) => {
      const updated = { ...current }
      if (next === undefined) delete updated[key]
      else updated[key] = next
      return updated
    })

  return (
    <Pane
      label={t('score.newTitle')}
      bodyClassName="flex flex-col gap-4 p-4"
      head={<span className="truncate font-mono text-xs text-faint">{t('score.newTitle')}</span>}
      foot={
        <ScoreFoot
          total={filled === 0 ? null : preview}
          tier={tierFor(tiers, preview)}
          tiers={tiers}
          ahead={ahead}
          trail={trail}
          work={work}
          scored={scored}
          action={
            <Button
              variant="primary"
              disabled={filled === 0 || save.isPending}
              onClick={() => save.mutate()}
            >
              {t('score.save')}
            </Button>
          }
        />
      }
    >
      <AxisGrid empty={axes.length === 0}>
        {axes.map((axis) => {
          // Drawn only where it is true: the mark on THIS axis from which
          // the total would cross into the tier ahead. No such mark, no line.
          const crossing =
            ahead === undefined ? undefined : markReaching(axes, values, axis, ahead.tier.min)
          return (
            <AxisRow
              key={axis.key}
              axis={axis}
              value={values[axis.key]}
              onChange={mark(axis.key)}
              trend={trend(axis.key)}
              threshold={
                crossing === undefined || ahead === undefined
                  ? undefined
                  : {
                      mark: crossing,
                      label: t('score.crossesHere', { tier: sayLabel(ahead.tier.label) }),
                    }
              }
            />
          )
        })}
      </AxisGrid>

      {filled > 0 && filled < axes.length && (
        <p className="text-xs text-dim">{t('score.partial', { filled, count: axes.length })}</p>
      )}

      <div className="flex flex-wrap items-end gap-3 border-t border-line pt-4">
        {/* A score belongs to the draft it judged - the current one unless
            another is picked. A work with no draft yet has nothing to pick. */}
        {judged.length > 0 && (
          // A group rather than a Field: the select is a button and a popup,
          // not an input a Field could hand its id to.
          <FieldGroup label={t('score.ofVersion')}>
            <Select
              className="w-64"
              aria-label={t('score.ofVersion')}
              value={versionId}
              onChange={setChosen}
              options={judged.map((version) => ({
                value: version.id,
                label: `${labelOf(roles, version.role)} · ${
                  version.label ?? t('versions.revision', { number: version.revision })
                }${version.is_current ? ` · ${t('versions.current')}` : ''}`,
              }))}
            />
          </FieldGroup>
        )}

        {/* Who is judging. Empty means the author, which is what the column
            has meant since v0.50 - so a second opinion is a name typed here,
            not a second workspace. */}
        <Field label={t('score.rater')}>
          <Input
            className="w-44"
            value={rater}
            onChange={(event) => onRater(event.target.value)}
            placeholder={t('score.raterHint')}
          />
        </Field>

        <Field label={t('score.note')} className="min-w-56 flex-1">
          <Input
            value={note}
            onChange={(event) => setNote(event.target.value)}
            placeholder={t('score.notePlaceholder')}
          />
        </Field>
      </div>
    </Pane>
  )
}
