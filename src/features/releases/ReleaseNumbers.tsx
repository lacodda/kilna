import { useTranslation } from 'react-i18next'
import { formatNumber, formatWeekday } from '@/lib/format'

interface Props {
  /** The release's `meta`, which carries its numbers under `analytics`. */
  meta: Record<string, unknown>
  className?: string
}

/** A number of the analytics, when it is one. */
function numberOf(analytics: Record<string, unknown>, key: string): number | null {
  const value = analytics[key]
  return typeof value === 'number' && Number.isFinite(value) ? value : null
}

/**
 * How a release did, as YouTube Studio's export said when it was last read
 * (v0.160, FR-22 22.1 p.2): views, click-through, how much was watched,
 * likes and subscribers, and the day it was read - numbers go stale, and a
 * reader should know how stale.
 */
export function ReleaseNumbers({ meta, className }: Props) {
  const { t } = useTranslation()
  const analytics = meta.analytics
  if (analytics === null || typeof analytics !== 'object') return null
  const numbers = analytics as Record<string, unknown>
  const views = numberOf(numbers, 'views')
  const ctr = numberOf(numbers, 'ctr')
  const watched = numberOf(numbers, 'avg_view_pct')
  const likes = numberOf(numbers, 'likes')
  const subscribers = numberOf(numbers, 'subscribers')
  const readAt = typeof numbers.read_at === 'string' ? numbers.read_at : null
  const parts = [
    views === null
      ? null
      : t('analytics.views', { count: Math.round(views), n: formatNumber(views, 0) }),
    ctr === null ? null : t('analytics.ctr', { value: formatNumber(ctr, 1) }),
    watched === null ? null : t('analytics.watched', { value: formatNumber(watched, 0) }),
    likes === null ? null : t('analytics.likes', { n: formatNumber(likes, 0) }),
    subscribers === null ? null : t('analytics.subscribers', { n: formatNumber(subscribers, 0) }),
  ].filter((part) => part !== null)
  if (parts.length === 0) return null
  return (
    <p className={className ?? 'px-3 pb-2 text-xs text-dim'}>
      {parts.join(' · ')}
      {readAt !== null && (
        <span className="text-faint">
          {' · '}
          {t('analytics.readAt', { day: formatWeekday(readAt) })}
        </span>
      )}
    </p>
  )
}
