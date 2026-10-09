import { useTranslation } from 'react-i18next'
import { AudioLines, Check, CornerDownRight, Dot, Flag, TriangleAlert, X } from 'lucide-react'
import type { TrialCard } from '@/lib/api/types'
import { opening, type SeriesRows } from '@/lib/trials'
import { cn } from '@/lib/utils'
import { RowButton } from '@/components/ui/list-row'
import { Skeleton } from '@/components/ui/skeleton'

interface Props {
  series: SeriesRows[]
  selected: string | null
  onSelect: (id: string) => void
  /** Trials that arrived while the board was watched. */
  fresh: ReadonlySet<string>
  /** Rows to draw while a sweep is being written, at the foot of the list. */
  coming: number
}

/**
 * The trials of a board as the list beside the open one draws them (v0.95):
 * series by series, each a tree - a variation indented under the trial it
 * varies - and every row saying at a glance what it is: its verdict, what it
 * moves, an anchor lost, the takes it was heard by, whether it went anywhere.
 */
export function TrialList({ series, selected, onSelect, fresh, coming }: Props) {
  const { t } = useTranslation()
  return (
    <div className="flex flex-col gap-3 p-1.5">
      {series.map((group) => (
        <section key={group.series} aria-label={group.series || t('trials.noSeries')}>
          <h3 className="caption px-2.5 pb-1">{group.series || t('trials.noSeries')}</h3>
          <ul className="flex flex-col">
            {group.rows.map(({ card, depth }) => (
              <li key={card.trial.id} style={{ paddingLeft: `${Math.min(depth, 6) * 14}px` }}>
                <TrialRow
                  card={card}
                  child={depth > 0}
                  selected={card.trial.id === selected}
                  fresh={fresh.has(card.trial.id)}
                  onSelect={() => onSelect(card.trial.id)}
                />
              </li>
            ))}
          </ul>
        </section>
      ))}
      {Array.from({ length: coming }, (_, index) => (
        <Skeleton key={index} className="mx-2.5 h-8" />
      ))}
    </div>
  )
}

function TrialRow({
  card,
  child,
  selected,
  fresh,
  onSelect,
}: {
  card: TrialCard
  child: boolean
  selected: boolean
  fresh: boolean
  onSelect: () => void
}) {
  const { t } = useTranslation()
  const { trial } = card
  const words = trial.angle.trim() || opening(trial.body) || t('trials.empty')
  const verdict =
    trial.verdict === 'keep' ? (
      <Check aria-label={t('trials.verdict.keep')} className="text-good" />
    ) : trial.verdict === 'drop' ? (
      <X aria-label={t('trials.verdict.drop')} />
    ) : (
      <Dot aria-label={t('trials.verdict.open')} />
    )
  return (
    <RowButton
      selected={selected}
      onClick={onSelect}
      className={cn(trial.verdict === 'drop' && 'opacity-60')}
      start={
        <span className="flex items-center gap-0.5">
          {child && <CornerDownRight aria-hidden className="size-3 text-faint" />}
          {verdict}
        </span>
      }
      description={trial.angle.trim() === '' ? undefined : opening(trial.body)}
      end={
        <>
          {fresh && (
            <span className="rounded-sm bg-accent-soft px-1 text-2xs text-accent">
              {t('trials.fresh')}
            </span>
          )}
          {trial.run_first && (
            <Flag aria-label={t('trials.runFirst')} className="size-3.5 text-accent" />
          )}
          {card.lost_anchors.length > 0 && (
            <TriangleAlert
              aria-label={t('trials.lostAnchors', { anchors: card.lost_anchors.join(', ') })}
              className="size-3.5 text-warn"
            />
          )}
          {card.takes.length > 0 && (
            <span
              className="flex items-center gap-0.5"
              title={t('trials.takes', { count: card.takes.length })}
            >
              <AudioLines aria-hidden className="size-3.5" />
              {card.takes.length}
            </span>
          )}
          {card.harvest.length > 0 && (
            <span
              className="text-good"
              title={t('trials.harvested', { count: card.harvest.length })}
            >
              →{card.harvest.length}
            </span>
          )}
        </>
      }
    >
      {words}
    </RowButton>
  )
}
