import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { catalogue as fetchCatalogue } from '@/lib/api'
import { keys } from '@/lib/query'
import { labelOf, useProfile } from '@/lib/useProfile'
import { EmptyState } from '@/components/ui/EmptyState'
import { SkeletonList } from '@/components/ui/Skeleton'
import { cn } from '@/lib/utils'

interface Props {
  onSelect: (workId: string) => void
}

// Everything judged, best first — the view that answers "what deserves to ship".
export function Catalogue({ onSelect }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()

  const rows = useQuery({
    queryKey: keys.catalogue,
    queryFn: fetchCatalogue,
  })

  if (rows.isPending) {
    return <SkeletonList rows={6} />
  }

  if (rows.isError) {
    return (
      <p role="alert" className="text-sm text-bad">
        {t('toast.loadFailed')}
      </p>
    )
  }

  if (rows.data.length === 0) {
    return <EmptyState title={t('empty.catalogueTitle')} body={t('empty.catalogueBody')} />
  }

  return (
    <table className="w-full text-sm">
      <thead>
        <tr className="border-b border-line text-left text-xs uppercase tracking-wide text-dim">
          <th className="py-2 pr-3 font-medium">{t('catalogue.work')}</th>
          <th className="py-2 pr-3 font-medium">{t('catalogue.tier')}</th>
          <th className="py-2 pr-3 text-right font-medium">{t('catalogue.total')}</th>
          <th className="py-2 font-medium">{t('catalogue.scored')}</th>
        </tr>
      </thead>
      <tbody>
        {rows.data.map((row) => (
          <tr
            key={row.work_id}
            className="cursor-pointer border-b border-line hover:bg-soft"
            onClick={() => onSelect(row.work_id)}
          >
            <td className="py-2 pr-3">
              <span className="font-medium">{row.title}</span>
              <span className="ml-2 text-xs text-dim">
                {labelOf(profile.config.statuses, row.status)}
              </span>
            </td>
            <td className="py-2 pr-3">
              {row.tier === null ? (
                <span className="text-faint">—</span>
              ) : (
                <span className="rounded bg-accent-soft px-1.5 py-0.5 text-xs">
                  {labelOf(profile.config.tiers, row.tier)}
                </span>
              )}
            </td>
            <td className={cn('py-2 pr-3 text-right tabular-nums', row.total === null && 'text-faint')}>
              {row.total?.toFixed(1) ?? '—'}
            </td>
            <td className="py-2 text-xs text-dim">
              {row.scored_at?.slice(0, 10) ?? '—'}
              {row.stale && (
                <span
                  className="ml-2 rounded bg-warn-soft px-1.5 py-0.5 text-warn"
                  title={t('catalogue.staleHint')}
                >
                  {t('catalogue.stale')}
                </span>
              )}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  )
}
