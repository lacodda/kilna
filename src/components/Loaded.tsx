import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import type { UseQueryResult } from '@tanstack/react-query'
import { QueryState } from '@/components/ui/query-state'
import { humanError } from '@/lib/errors'
import { cn } from '@/lib/utils'

/*
 * One query's ladder - loading, failed, nothing there, the content - in
 * kilna's words.
 *
 * dowel's QueryState takes values rather than a query, so the registry does
 * not carry TanStack into products that fetch some other way. kilna fetches
 * one way, and nineteen screens wrote the same failure by hand: "could not
 * load" in red, with the query's own `refetch` in hand and never called. This
 * is the one place that hands the query over, so every failed load says what
 * failed and offers to try again, and every screen waiting for data says so
 * the same way.
 *
 * The content is a function of the data, so it is only built once the data
 * is there and TypeScript knows it is.
 */
interface LoadedProps<T> {
  query: UseQueryResult<T>
  children: (data: T) => ReactNode
  /** What stands in while it loads: the shape of what is coming. */
  skeleton?: ReactNode
  /** Whether what arrived is nothing. Only the caller knows what empty means. */
  isEmpty?: (data: T) => boolean
  /** The invitation shown when it is: what would be here, and how to add it. */
  emptyState?: ReactNode
  /** The failure without its frame, inside a panel that already has one. */
  plain?: boolean
  /**
   * Hand the height down to the content, for content that lays itself out
   * against it - a month grid, a table with a sticky header. QueryState wraps
   * whatever it shows in a bare `<div>`, which would end the chain of
   * `min-h-0 flex-1` columns there; this makes that wrapper one of them.
   */
  fill?: boolean
}

export function Loaded<T>({
  query,
  children,
  skeleton,
  isEmpty,
  emptyState,
  plain,
  fill = false,
}: LoadedProps<T>) {
  const { t } = useTranslation()

  const ladder = (
    <QueryState
      pending={query.isPending}
      error={query.isError ? humanError(query.error) : null}
      empty={query.data !== undefined && isEmpty?.(query.data) === true}
      skeleton={skeleton}
      emptyState={emptyState}
      errorLabels={{ title: t('toast.loadFailed') }}
      plain={plain}
      onRetry={() => void query.refetch()}
      retryLabel={t('crash.retry')}
    >
      {query.data === undefined ? null : children(query.data)}
    </QueryState>
  )

  if (!fill) return ladder

  return (
    <div
      className={cn(
        'flex min-h-0 min-w-0 flex-1 flex-col',
        '*:flex *:min-h-0 *:min-w-0 *:flex-1 *:flex-col',
      )}
    >
      {ladder}
    </div>
  )
}
