import { cn } from '@/lib/utils'

/**
 * A placeholder block shaped like the thing still loading.
 *
 * Skeletons stand in for content whose shape we already know. A spinner is the
 * right answer when we do not — it promises nothing about what arrives.
 */
export function Skeleton({ className }: { className?: string }) {
  return <div className={cn('animate-pulse rounded-[9px] bg-soft', className)} aria-hidden />
}

/** Rows for a list of titles with a subtitle underneath — the works list shape. */
export function SkeletonList({ rows = 5, className }: { rows?: number; className?: string }) {
  return (
    <div className={cn('flex flex-col gap-1', className)} aria-hidden>
      {Array.from({ length: rows }, (_, row) => (
        <div key={row} className="flex flex-col gap-1.5 px-3 py-2">
          {/* Uneven widths so it reads as text, not as a loading bar. */}
          <Skeleton className={cn('h-3.5', row % 3 === 0 ? 'w-2/3' : row % 3 === 1 ? 'w-4/5' : 'w-1/2')} />
          <Skeleton className="h-2.5 w-1/3" />
        </div>
      ))}
    </div>
  )
}

/** Blocks roughly the size of the card's stacked panels. */
export function SkeletonCard({ className }: { className?: string }) {
  return (
    <div className={cn('flex flex-col gap-6', className)} aria-hidden>
      <div className="flex flex-wrap items-end gap-3">
        <Skeleton className="h-9 w-64" />
        <Skeleton className="h-9 w-44" />
        <Skeleton className="h-9 w-44" />
      </div>
      <Skeleton className="h-32 w-full" />
      <Skeleton className="h-24 w-full" />
    </div>
  )
}
