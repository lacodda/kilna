import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { keepAnalytics, readAnalytics } from '@/lib/api/analytics'
import { humanError } from '@/lib/errors'
import { formatNumber } from '@/lib/format'
import { keys } from '@/lib/query/keys'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { Dialog } from '@/components/AppDialog'
import { Button } from '@/components/ui/button'

interface Props {
  /** The export chosen: a CSV from YouTube Studio's Advanced mode. */
  path: string
  onClose: () => void
}

/**
 * YouTube Studio's export read against the space's releases (v0.160,
 * FR-22 22.1 p.2): each row with the release it belongs to - found by the
 * video's id in the release's link, else by its exact title - and its
 * numbers, before anything is kept. One press keeps them all, one undo
 * takes them back.
 */
export function AnalyticsImportDialog({ path, onClose }: Props) {
  const { t } = useTranslation()
  const read = useQuery({
    queryKey: ['analytics', 'read', path],
    queryFn: () => readAnalytics(path),
  })
  const rows = read.data?.rows ?? []
  const matched = rows.filter((row) => row.release_id !== null)

  const keep = useAppMutation({
    mutationFn: () => keepAnalytics(path),
    failure: 'analytics.keepFailed',
    refresh: [...refresh.works, keys.calendar],
    onSuccess: (kept) => {
      say.ok(t('analytics.kept', { count: kept }))
      onClose()
    },
  })

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) onClose()
      }}
      title={t('analytics.title')}
      size="lg"
      footer={
        <Button
          variant="primary"
          disabled={matched.length === 0 || keep.isPending}
          onClick={() => keep.mutate()}
        >
          {t('analytics.keep', { count: matched.length })}
        </Button>
      }
    >
      {read.isError ? (
        <p className="text-sm text-bad">{humanError(read.error)}</p>
      ) : read.data === undefined ? null : (
        <div className="flex flex-col gap-2">
          <p className="text-sm text-dim">
            {t('analytics.summary', { matched: matched.length, total: rows.length })}
          </p>
          <ul className="flex flex-col gap-0.5 text-xs">
            {rows.map((row) => (
              <li key={row.line} className="flex flex-wrap items-baseline gap-1.5">
                <b
                  className={row.release_id === null ? 'font-semibold text-faint' : 'font-semibold'}
                >
                  {row.title ?? row.video_id ?? t('analytics.line', { line: row.line })}
                </b>
                <span className="text-dim">
                  {row.release_id === null
                    ? t('analytics.unmatched')
                    : t(`analytics.by.${row.by ?? 'title'}`, { work: row.work_title ?? '' })}
                </span>
                {row.metrics.views !== undefined && (
                  <span className="font-mono text-faint">
                    {t('analytics.views', {
                      count: Math.round(row.metrics.views),
                      n: formatNumber(row.metrics.views, 0),
                    })}
                  </span>
                )}
              </li>
            ))}
          </ul>
        </div>
      )}
    </Dialog>
  )
}
