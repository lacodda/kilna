import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import type { Score, Work } from '@/lib/api/types'
import { formatDay } from '@/lib/format'
import { toNextTier } from '@/lib/scoring'
import { useVocabulary } from '@/lib/useProfile'
import { KeyValue, KeyValueRow } from '@/components/ui/key-value'
import { Pane } from '@/components/frame'
import { AxisGrid, AxisRow } from '@/features/work/tabs/score/AxisRow'
import { KindVerdicts } from '@/features/work/tabs/score/KindVerdicts'
import { ScoreFoot } from '@/features/work/tabs/score/ScoreFoot'

interface Props {
  workId: string
  score: Score
  /** The newest score: the verdict that stands, which the channels read. */
  standing: boolean
  /** What it is called in the list: "Score of Lyrics v2". */
  title: string
  /** The draft it judged, said in full, when that draft is still there. */
  judged: string | undefined
  trail: Score[]
  /** An axis over the recorded scores, oldest first. */
  trend: (axis: string) => number[] | undefined
  work: Work | undefined
  /** The tier the standing score arrives at, for the pin. */
  scored: string | null
}

/**
 * A recorded score, read.
 *
 * Nothing here moves. Until v0.80 the scales of a recorded score were the
 * form for the next one - moving a mark on the verdict you were reading
 * started a new score from it - so reading and judging were one state, and
 * which one the panel was in had to be said in a line of small print. Now a
 * reading is a reading: the marks as meters, what was said about them, who
 * said it and about which draft. Judging again is "New score", with empty
 * scales.
 */
export function ScoreReading({
  workId,
  score,
  standing,
  title,
  judged,
  trail,
  trend,
  work,
  scored,
}: Props) {
  const { t } = useTranslation()
  const { axes, tiers } = useVocabulary(workId)

  // The marks restricted to axes the profile still has: a score taken before
  // an axis was removed keeps that mark, but the scale for it is gone.
  const marks: Record<string, number> = {}
  for (const axis of axes) {
    const mark = score.axes[axis.key]
    if (typeof mark === 'number') marks[axis.key] = mark
  }

  const heading = `${title} · ${formatDay(score.scored_at)}`

  return (
    <Pane
      label={heading}
      bodyClassName="flex flex-col gap-4 p-4"
      head={
        <>
          <span className="truncate font-mono text-xs text-faint">{heading}</span>
          {score.version_id !== null && (
            // The judgement points at what was judged: the review of that
            // draft is the rest of the sentence this number starts.
            <Link
              to={`/works/${workId}/versions?version=${score.version_id}`}
              className="ml-auto shrink-0 text-xs text-dim underline decoration-dotted underline-offset-2 transition-colors hover:text-text"
            >
              {t('score.openVersion')}
            </Link>
          )}
        </>
      }
      foot={
        <ScoreFoot
          // The recorded total and tier, not the ones these marks would make
          // today: the backend weighed them when the score was given, and a
          // profile reweighed since does not rewrite what was said then.
          total={score.total}
          tier={tiers.find((tier) => tier.key === score.tier)}
          tiers={tiers}
          ahead={toNextTier(axes, marks, tiers, score.total)}
          trail={trail}
          openId={score.id}
          work={work}
          scored={scored}
        />
      }
    >
      <AxisGrid empty={axes.length === 0}>
        {axes.map((axis) => (
          <AxisRow key={axis.key} axis={axis} value={marks[axis.key]} trend={trend(axis.key)} />
        ))}
      </AxisGrid>

      <KeyValue className="border-t border-line pt-4">
        {judged !== undefined && <KeyValueRow label={t('score.judged')}>{judged}</KeyValueRow>}
        {/* Null has meant the author since v0.50: your own verdict says so
            rather than leaving the line blank. */}
        <KeyValueRow label={t('score.rater')}>
          {score.rater !== null && score.rater !== '' ? score.rater : t('score.raterSelf')}
        </KeyValueRow>
        {score.note !== null && score.note !== '' && (
          <KeyValueRow label={t('score.note')}>{score.note}</KeyValueRow>
        )}
      </KeyValue>

      {/* What the standing score means to each channel. Only under the one
          that stands: the verdicts are read off the latest score, and under
          an older one they would be answering a different question. */}
      {standing && <KindVerdicts workId={workId} />}
    </Pane>
  )
}
