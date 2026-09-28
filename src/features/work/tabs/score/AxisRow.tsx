import { type ReactNode, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import type { Axis } from '@/lib/api/types'
import { formatNumber } from '@/lib/format'
import { rubricFor } from '@/lib/scoring'
import { say as sayLabel } from '@/lib/useProfile'
import { Sparkline } from '@/components/ui/sparkline'
import { AxisBar } from '@/components/ui/tier'

interface Props {
  axis: Axis
  /** The mark on this axis, or `undefined` while it is unjudged. */
  value: number | undefined
  /** Given, the scale is a control; omitted, it is a reading of a score that
   * was recorded - a meter, not a disabled slider. */
  onChange?: (value: number | undefined) => void
  /** The mark from which the total crosses into the next tier, when one does. */
  threshold?: { mark: number; label: string }
  /** This axis over the recorded scores, oldest first. Two points make a line;
   * fewer, or none while judging blind, and nothing is drawn. */
  trend?: number[]
}

/**
 * One axis of a score: what it is called, what it weighs and asks, the
 * scale, and what the mark on it means.
 *
 * The same row reads a recorded score and gives a new one - the only
 * difference is whether the scale answers the pointer. Kept one component so
 * the two states line up axis for axis, and the eye moving from a reading to
 * the form it is about to fill finds every scale where it was.
 */
export function AxisRow({ axis, value, onChange, threshold, trend }: Props) {
  const { t } = useTranslation()
  // The mark the pointer is over, so the rubric can answer "what is a seven
  // here" while the person is deciding rather than after.
  const [hovered, setHovered] = useState<number | undefined>(undefined)

  const label = sayLabel(axis.label)
  const description = sayLabel(axis.description)
  // The mark being hovered wins over the one already set: the question while
  // scoring is about the mark being weighed, not the one already given.
  const weighed = hovered ?? value
  const rubric = weighed === undefined ? undefined : rubricFor(axis, weighed)

  // Three cells straight into the grid of `AxisGrid`, not a grid of their
  // own: a row that sizes its own columns puts every scale at a different
  // width as soon as one axis has a trend line and the next has none.
  return (
    <>
      <span className="min-w-0">
        <b className="block truncate text-sm font-semibold">
          {label}{' '}
          <span className="font-mono text-2xs font-normal text-faint">
            ×{formatNumber(axis.weight, 1)}
          </span>
        </b>
        {/* The question the axis asks, in the open. It used to be a tooltip,
            which is the same as not being there - but wrapping it made a
            six-axis card taller than the screen, and scoring is a judgement
            you make by looking at all the axes at once. One line, with the
            whole of it on hover. */}
        {description !== '' && (
          <span className="block truncate text-xs text-faint" title={description}>
            {description}
          </span>
        )}
      </span>

      <span className="flex min-w-0 flex-col gap-1">
        <AxisBar
          scale={axis.scale}
          value={value}
          label={label}
          valueText={value === undefined ? t('score.unjudged') : String(value)}
          threshold={threshold}
          onPreview={setHovered}
          onChange={onChange}
        />

        {/* What the mark under consideration means, when the craft has said.
            The line keeps its row whether or not it has anything to say.
            Appearing on hover pushed the axes below out from under the
            pointer, the hover ended, the line went, the axes came back under
            the pointer - a strobe. */}
        <span
          className="block h-4 truncate text-xs leading-4 text-dim"
          title={rubric === undefined ? undefined : sayLabel(rubric.label)}
        >
          {rubric === undefined ? (
            ' '
          ) : (
            <>
              <b className="font-mono font-semibold">{rubric.at}</b> — {sayLabel(rubric.label)}
            </>
          )}
        </span>
      </span>

      <span className="flex items-center justify-end gap-2">
        {/* This axis over time, beside the axis it belongs to. The trail at
            the foot says the card moved; these say which axis moved it. */}
        {trend !== undefined && trend.length > 1 && (
          <Sparkline
            values={trend}
            max={axis.scale}
            size="sm"
            label={t('score.axisTrend', {
              axis: label,
              from: formatNumber(trend[0]!, 0),
              to: formatNumber(trend.at(-1)!, 0),
            })}
          />
        )}

        <span className="w-6 text-right font-mono text-sm text-dim tabular-nums">
          {value ?? '—'}
        </span>
      </span>
    </>
  )
}

/** The axes of one score, their names, scales and marks each in one column. */
export function AxisGrid({ empty, children }: { empty: boolean; children: ReactNode }) {
  const { t } = useTranslation()

  // A kind with no axes yet - a video in a workspace whose owner has not
  // written its judgement - is scored empty rather than on the song's axes.
  // Said here, with the way to the editor, because an empty panel reads as
  // broken and it is not.
  if (empty) {
    return (
      <p className="text-sm text-dim">
        {t('score.noAxes')}{' '}
        <Link
          to="/settings"
          className="underline decoration-dotted underline-offset-2 hover:text-text"
        >
          {t('score.noAxesLink')}
        </Link>
      </p>
    )
  }

  return (
    <div className="grid items-center gap-x-3 gap-y-2.5 sm:grid-cols-[minmax(10rem,12rem)_minmax(0,1fr)_auto]">
      {children}
    </div>
  )
}
