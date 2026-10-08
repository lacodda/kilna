import { useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import type { Collection, ScoredWork } from '@/lib/api/types'
import { progressOf, standingOf } from '@/lib/collections'
import { formatTotal } from '@/lib/format'
import { labelOf, useProfile } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Chip } from '@/components/ui/chip'
import { Progress } from '@/components/ui/progress'
import { GoalLine } from './GoalLine'

interface Props {
  collection: Collection
  /** The catalogue's rows, for the verdict of what it holds. */
  rows: readonly ScoredWork[]
  today: string
  /** It is the one open beside the grid. */
  open: boolean
  onOpen: () => void
}

/**
 * One collection of the grid, as the mockup draws it: its kind and how full
 * it is across the top, its name, the bar to its goal and the day, then the
 * verdict of what it holds - the mean and the weakest link.
 *
 * The weakest link rather than the strongest: an album is as good as the
 * track that makes someone skip, and the card is read to decide what to work
 * on next. The page beside it names both.
 */
export function CollectionCard({ collection, rows, today, open, onOpen }: Props) {
  const { t } = useTranslation()
  const { config } = useProfile()
  const card = useRef<HTMLLIElement>(null)
  const progress = progressOf(collection, today)
  const standing = standingOf(collection, rows)
  const unjudged = standing.works.length - standing.scored

  // The open collection narrows the grid to a column; its card is kept in
  // view there, so the column says which one is open.
  useEffect(() => {
    if (open) card.current?.scrollIntoView({ block: 'nearest' })
  }, [open])

  return (
    <li ref={card} className="flex">
      {/* A quiet Button in the shape of a card, as a style's is: the bar and
          the chips do not fit a row's slots. No size, because a card is as
          tall as what it holds. */}
      <Button
        variant="ghost"
        size={null}
        onClick={onOpen}
        aria-current={open ? 'true' : undefined}
        className={cn(
          'w-full flex-col items-stretch justify-start gap-2 rounded-lg bg-raise px-3 pt-2.5 pb-3 text-left font-normal whitespace-normal',
          open && 'border-accent',
        )}
      >
        <span className="flex min-w-0 items-center gap-2">
          <span className="min-w-0 flex-1 truncate caption">
            {labelOf(config.collection_kinds, collection.kind)}
          </span>
          <Chip
            variant={progress.target === null ? 'outline' : progress.met ? 'good' : 'warn'}
            className="shrink-0 font-mono tabular-nums"
          >
            {progress.target === null
              ? progress.held
              : t('collections.ofTarget', { held: progress.held, target: progress.target })}
          </Chip>
        </span>

        <span className="truncate text-base font-semibold text-text">{collection.title}</span>

        {progress.fraction !== null && (
          <Progress
            size="sm"
            tone={progress.met ? 'good' : 'warn'}
            value={progress.fraction * 100}
            label={t('collections.progressLabel', { title: collection.title })}
          />
        )}
        <GoalLine progress={progress} className="text-xs text-faint" />

        {(standing.average !== null || unjudged > 0) && (
          <span className="flex flex-wrap items-center gap-1.5">
            {standing.average !== null && (
              <Chip>
                {t('collections.average')}{' '}
                <span className="font-mono tabular-nums">{formatTotal(standing.average)}</span>
              </Chip>
            )}
            {standing.weakest !== null && standing.weakest.total !== null && (
              <Chip variant="warn" className="min-w-0" title={standing.weakest.title}>
                <span className="truncate">
                  {t('collections.weakest', { title: standing.weakest.title })}
                </span>
                <span className="font-mono tabular-nums">
                  {formatTotal(standing.weakest.total)}
                </span>
              </Chip>
            )}
            {unjudged > 0 && (
              <Chip variant="dashed">{t('collections.unjudged', { count: unjudged })}</Chip>
            )}
          </span>
        )}
      </Button>
    </li>
  )
}
