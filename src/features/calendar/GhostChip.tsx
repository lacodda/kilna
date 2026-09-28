import type { Ghost } from '@/lib/layout'
import { accentFor } from '@/lib/cover'
import { KindGlyph } from '@/lib/releaseIcon'
import { allOf, labelOf, useProfile } from '@/lib/useProfile'

/**
 * Where the auto-layout would put a queued release: the booked chip's one
 * line, dashed and in the work's own colour as an outline, so a plan is
 * visibly not a booking.
 *
 * Inert on purpose - the plan is approved or cancelled from the bar above the
 * month, not edited chip by chip.
 */
export function GhostChip({ ghost }: { ghost: Ghost }) {
  const profile = useProfile()
  const releaseKinds = allOf(profile.config, 'release_kinds')
  const kind = releaseKinds.find((entry) => entry.key === ghost.kind)

  return (
    <div
      title={`${ghost.title} · ${labelOf(releaseKinds, ghost.kind)}`}
      className="flex h-5 w-full min-w-0 shrink-0 items-center gap-1 rounded-sm border border-dashed px-1.5 text-xs font-medium text-dim"
      style={{ borderColor: accentFor(ghost.workId) }}
    >
      <span className="min-w-0 flex-1 truncate">{ghost.title}</span>
      <KindGlyph icon={kind?.icon} className="size-3 shrink-0 text-faint" />
    </div>
  )
}
