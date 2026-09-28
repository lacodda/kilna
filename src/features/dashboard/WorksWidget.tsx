import { useTranslation } from 'react-i18next'
import type { ScoredWork } from '@/lib/api/types'
import { tally } from '@/lib/dashboard'
import { formatNumber } from '@/lib/format'
import { Chip } from '@/components/ui/chip'
import { Widget } from '@/features/dashboard/Widget'

/**
 * How big the catalogue is, and how much of it is judged and out.
 *
 * Read off the catalogue rows the screen already has - three counts are not
 * worth a question of their own to the backend.
 */
export function WorksWidget({ works }: { works: readonly ScoredWork[] }) {
  const { t } = useTranslation()
  const counts = tally(works)

  return (
    <Widget title={t('dashboard.works')}>
      <p className="flex items-baseline gap-2">
        <span className="font-mono text-2xl leading-none font-semibold tabular-nums">
          {formatNumber(counts.works, 0)}
        </span>
        <span className="text-xs text-faint">{t('dashboard.inCatalogue')}</span>
      </p>
      <div className="flex flex-wrap gap-1.5">
        <Chip variant="accent">{t('dashboard.scoredCount', { count: counts.scored })}</Chip>
        <Chip variant="warn">{t('dashboard.unscoredCount', { count: counts.unscored })}</Chip>
        <Chip variant="good">{t('dashboard.releasedCount', { count: counts.released })}</Chip>
      </div>
    </Widget>
  )
}
