import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { catalogue as fetchCatalogue, type ScoredWork } from '@/lib/api'
import { labelOf, useProfile } from '@/lib/useProfile'
import { cn } from '@/lib/utils'

interface Props {
  revision: number
  onSelect: (workId: string) => void
}

// Everything judged, best first — the view that answers "what deserves to ship".
export function Catalogue({ revision, onSelect }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const [rows, setRows] = useState<ScoredWork[]>([])
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    fetchCatalogue()
      .then((result) => {
        setRows(result)
        setError(null)
      })
      .catch((cause: unknown) => setError(String(cause)))
  }, [revision])

  if (error !== null) {
    return (
      <p role="alert" className="text-sm text-bad">
        {error}
      </p>
    )
  }

  if (rows.length === 0) {
    return <p className="py-8 text-center text-sm text-dim">{t('works.none')}</p>
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
        {rows.map((row) => (
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
