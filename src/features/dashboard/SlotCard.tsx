import type { Slot } from '@/lib/dashboard'
import { coverImageFor } from '@/lib/cover'
import { formatDay } from '@/lib/format'
import { useCovers } from '@/lib/useCovers'
import { allOf, labelOf, useProfile } from '@/lib/useProfile'
import { TileButton } from '@/components/ui/tile-button'
import { WorkRepeatMark } from '@/components/RepeatMark'
import { WorkVibeMark } from '@/components/WorkVibeMark'

/**
 * One of the nearest slots: the cover, the work, what goes out and when.
 *
 * Opens on the work's releases, where the slot it stands for is kept - the
 * card is about a date, not about the work in general.
 */
export function SlotCard({
  slot,
  onSelect,
}: {
  slot: Slot
  onSelect: (workId: string, tab?: string) => void
}) {
  const covers = useCovers()
  const profile = useProfile()
  const { release } = slot

  return (
    // A card, not a row: a cover over two lines, which no row's slots hold.
    // So a tile pressed whole, whose border and hover are the card's.
    <TileButton
      onClick={() => onSelect(release.work_id, 'overview')}
      className="rounded-lg bg-raise"
    >
      <span
        aria-hidden
        className="block h-16.5"
        style={{ background: coverImageFor(release.work_id, covers.get(release.work_id)) }}
      />
      <span className="block px-3 pt-2 pb-2.5">
        {/* The title and the guard's mark of its song (ADR 0054) on one line:
            the mark is about the work the title names. */}
        <span className="flex items-center gap-1.5">
          <span className="min-w-0 truncate text-sm font-semibold text-text">
            {release.work_title}
          </span>
          <WorkVibeMark shown={release.work_vibe} />
          <WorkRepeatMark workId={release.work_id} />
        </span>
        <span className="block truncate font-mono text-xs text-faint">
          {labelOf(allOf(profile.config, 'release_kinds'), release.kind)} ·{' '}
          {formatDay(release.scheduled_at as string)}
        </span>
      </span>
    </TileButton>
  )
}
