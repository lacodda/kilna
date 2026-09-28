import { Skeleton } from '@/components/ui/skeleton'
import { cn } from '@/lib/utils'

/** Five weeks: what most months draw once the neighbours fill them out. */
const CELLS = 35

/**
 * The month's own shape while the calendar loads: the weekday row over weeks
 * that share the height, in the panel the month is drawn in.
 *
 * A list of rows stood here until v0.43, and it was itself the jump it was
 * meant to cover. A skeleton that is the wrong shape is worse than none: it
 * promises something the content does not keep. The title and its arrows are
 * the screen's head and stand while this loads, so they are not drawn here.
 */
export function MonthSkeleton() {
  return (
    <div
      aria-hidden
      className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden rounded-lg border border-line bg-raise"
    >
      <div className="grid shrink-0 grid-cols-7 border-b border-line">
        {Array.from({ length: 7 }, (_, day) => (
          <div key={day} className="px-2 py-2">
            <Skeleton className="h-2 w-6" />
          </div>
        ))}
      </div>
      <div className="grid min-h-0 flex-1 auto-rows-fr grid-cols-7">
        {Array.from({ length: CELLS }, (_, cell) => (
          <div
            key={cell}
            className={cn(
              'border-line p-1.5',
              cell % 7 !== 6 && 'border-r',
              cell < CELLS - 7 && 'border-b',
            )}
          >
            <Skeleton className="h-2 w-3" />
          </div>
        ))}
      </div>
    </div>
  )
}
