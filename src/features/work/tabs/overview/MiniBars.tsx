import type { Axis, Score } from '@/lib/api/types'
import { formatNumber } from '@/lib/format'
import { say as sayLabel } from '@/lib/useProfile'
import { Progress } from '@/components/ui/progress'

export interface Bar {
  key: string
  /** What the bar is of, short: the row's first column truncates it. */
  label: string
  value: number | undefined
  max: number
  /** The value as it is written at the row's end. */
  shown: string
}

/**
 * The mockup's `.mini`: a few short bars, one per line - a name, a track, a
 * figure. The widget catalogue's breakdown, a number taken apart along its
 * axes, read at a glance rather than judged: the Score tab has the scales.
 */
export function MiniBars({ bars }: { bars: readonly Bar[] }) {
  return (
    <ul className="flex flex-col gap-1">
      {bars.map((bar) => (
        <li
          key={bar.key}
          className="grid grid-cols-[minmax(0,5rem)_minmax(0,1fr)_1.5rem] items-center gap-2 text-xs"
        >
          <span className="truncate text-faint" title={bar.label}>
            {bar.label}
          </span>
          <Progress
            size="sm"
            value={bar.value ?? 0}
            max={bar.max}
            label={`${bar.label}: ${bar.shown}`}
          />
          <span className="text-right font-mono text-dim tabular-nums">{bar.shown}</span>
        </li>
      ))}
    </ul>
  )
}

/** One axis of a score as a bar: its name, its mark on its own scale. */
export function axisBar(axis: Axis, score: Score, weighted = false): Bar {
  const mark = score.axes[axis.key]
  const value = typeof mark === 'number' ? mark : undefined
  const name = sayLabel(axis.label)
  return {
    key: axis.key,
    // The weight beside the name where the bars are the widget's whole
    // subject, as the Score tab writes it: `Hook ×2.0`.
    label: weighted && axis.weight !== 1 ? `${name} ×${formatNumber(axis.weight, 1)}` : name,
    value,
    max: axis.scale,
    // A half mark is a mark of its own: 9.5 read as 10 claimed a verdict the
    // score did not give. Whole marks stay whole, as on the Score tab.
    shown: value === undefined ? '—' : formatNumber(value, Number.isInteger(value) ? 0 : 1),
  }
}
