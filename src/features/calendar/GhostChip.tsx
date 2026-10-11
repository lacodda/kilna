import type { Ghost } from '@/lib/layout'
import { accentFor } from '@/lib/cover'
import { KindGlyph } from '@/lib/releaseIcon'
import { allOf, labelOf, useProfile } from '@/lib/useProfile'
import { cn } from '@/lib/utils'

/**
 * Where the auto-layout would put a queued release: the booked chip's one
 * line, dashed and in the work's own colour as an outline, so a plan is
 * visibly not a booking.
 *
 * Inert on purpose - the plan is approved or cancelled from the bar above the
 * month, not edited chip by chip.
 */
export function GhostChip({ ghost, roomy = false }: { ghost: Ghost; roomy?: boolean }) {
  const profile = useProfile()
  const releaseKinds = allOf(profile.config, 'release_kinds')
  const kind = releaseKinds.find((entry) => entry.key === ghost.kind)

  return (
    <div
      title={`${ghost.title} · ${labelOf(releaseKinds, ghost.kind)}`}
      className={cn(
        'flex w-full min-w-0 shrink-0 gap-1 rounded-sm border border-dashed px-1.5 text-xs font-medium text-dim',
        // The week's two lines, as its booked chips have (`SlotChip`).
        roomy ? 'h-11 items-start py-1' : 'h-5 items-center',
      )}
      style={{ borderColor: accentFor(ghost.workId) }}
    >
      <span className={cn('min-w-0 flex-1', roomy ? 'line-clamp-2 leading-tight' : 'truncate')}>
        {ghost.title}
      </span>
      <KindGlyph icon={kind?.icon} className="size-3 shrink-0 text-faint" />
    </div>
  )
}
